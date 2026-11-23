//! Tauri shell: exposes the Rust diagnostic core to the React UI.
//!
//! Every command that changes something in the car goes through
//! [`Inner::gate`], which reads the car's live state and refuses to run when
//! any safety requirement is not proven. Every attempt, allowed or blocked,
//! is written to the activity log.

use protocol_core::bmw_e70::{self, JobStatus, Module};
use protocol_core::catalog::{self, Function, Requirement, RiskTier, VehicleState};
use protocol_core::obd2::{self, Dtc, MilStatus, PidInfo, Service};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tokio::sync::Mutex;
use transport::elm::{AdapterInfo, ClientError, ElmClient};
use transport::sim::{self, ElmSimulator, SimEngine};
use transport::{DiscoveredDevice, HardwareTransport, TransportKind};

type Client = ElmClient<Box<dyn HardwareTransport + Send>>;

#[derive(Default)]
struct Inner {
    client: Option<Client>,
    device: Option<DiscoveredDevice>,
    charger_confirmed: bool,
    log: Vec<LogEntry>,
}

#[derive(Default)]
struct AppState(Mutex<Inner>);

// ---- error type --------------------------------------------------------

#[derive(Debug, Serialize)]
struct CmdError {
    kind: &'static str,
    message: String,
    /// Safety requirements that blocked the command.
    unmet: Vec<RequirementView>,
}

impl CmdError {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            unmet: Vec::new(),
        }
    }
}

impl From<ClientError> for CmdError {
    fn from(e: ClientError) -> Self {
        use protocol_core::ProtocolError;
        let message = match &e {
            ClientError::Protocol(ProtocolError::NegativeResponse { nrc, .. }) => {
                protocol_core::diag::Nrc(*nrc).explain().to_string()
            }
            other => other.to_string(),
        };
        Self::new("device", message)
    }
}

type CmdResult<T> = Result<T, CmdError>;

// ---- views -------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
struct RequirementView {
    #[serde(flatten)]
    requirement: Requirement,
    text: String,
}

impl From<Requirement> for RequirementView {
    fn from(r: Requirement) -> Self {
        Self {
            text: r.explain(),
            requirement: r,
        }
    }
}

#[derive(Serialize)]
struct TierView {
    id: RiskTier,
    label: &'static str,
    explain: &'static str,
    confirmations: u8,
}

#[derive(Serialize)]
struct FunctionView {
    #[serde(flatten)]
    function: Function,
    tier_label: &'static str,
    confirmations: u8,
    requirements: Vec<RequirementView>,
    /// Whether the exact commands are verified for a real car.
    verified_on_car: bool,
}

#[derive(Serialize)]
struct Catalog {
    tiers: Vec<TierView>,
    functions: Vec<FunctionView>,
    pids: &'static [PidInfo],
    modules: &'static [Module],
}

#[derive(Serialize)]
struct LogEntry {
    at_ms: u128,
    action: String,
    target: String,
    tier: RiskTier,
    outcome: String,
    ok: bool,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

impl Inner {
    fn client(&mut self) -> CmdResult<&mut Client> {
        self.client
            .as_mut()
            .ok_or_else(|| CmdError::new("not_connected", "Connect to an adapter first."))
    }

    fn simulated(&self) -> bool {
        matches!(
            self.device.as_ref().map(|d| &d.transport),
            Some(TransportKind::Simulator { .. })
        )
    }

    fn record(
        &mut self,
        action: &str,
        target: &str,
        tier: RiskTier,
        ok: bool,
        outcome: impl Into<String>,
    ) {
        self.log.push(LogEntry {
            at_ms: now_ms(),
            action: action.into(),
            target: target.into(),
            tier,
            outcome: outcome.into(),
            ok,
        });
    }

    /// Reads what the gates need from the car. Anything that fails to read
    /// stays unknown, which blocks the gate.
    async fn vehicle_state(&mut self, function_id: Option<&str>) -> CmdResult<VehicleState> {
        let simulated = self.simulated();
        let charger = self.charger_confirmed;
        let c = self.client()?;
        let speed = c.read_pid(0x0D).await.ok();
        let rpm = c.read_pid(0x0C).await.ok();
        let battery = c.battery_voltage().await.ok().map(|v| v as f64);
        let job_verified = match function_id {
            _ if simulated => true,
            Some(id) => job_verified(id),
            None => false,
        };
        Ok(VehicleState {
            speed_kmh: speed,
            rpm,
            battery_v: battery,
            charger_confirmed: charger,
            wired_adapter: false,
            // The simulator takes its backup automatically before a write.
            backup_taken: simulated,
            job_verified,
        })
    }

    /// Refuses (and logs) when any requirement for `function_id` is unmet.
    async fn gate(&mut self, function_id: &str, target: &str) -> CmdResult<&'static Function> {
        let f = catalog::function(function_id)
            .ok_or_else(|| CmdError::new("unknown", format!("Unknown function {function_id}")))?;
        let state = self.vehicle_state(Some(function_id)).await?;
        let unmet = f.unmet(&state);
        if !unmet.is_empty() {
            let reasons: Vec<String> = unmet.iter().map(|r| r.explain()).collect();
            self.record(
                f.name,
                target,
                f.tier,
                false,
                format!("Blocked: {}", reasons.join(" ")),
            );
            return Err(CmdError {
                kind: "blocked",
                message: format!("{} is blocked until the car is ready.", f.name),
                unmet: unmet.into_iter().map(Into::into).collect(),
            });
        }
        Ok(f)
    }
}

/// True when every job behind a function has verified request bytes.
fn job_verified(function_id: &str) -> bool {
    match catalog::function(function_id).map(|f| f.tier) {
        Some(RiskTier::Read) => true,
        // Clearing codes uses standard services, not BMW-specific jobs.
        _ if function_id == "clear_codes" => true,
        _ => {
            let jobs: Vec<_> = bmw_e70::JOBS
                .iter()
                .filter(|j| j.function_id == function_id)
                .collect();
            !jobs.is_empty() && jobs.iter().all(|j| j.status == JobStatus::Verified)
        }
    }
}

// ---- commands: catalog and devices -------------------------------------

#[tauri::command]
fn catalog() -> Catalog {
    let tiers = [
        RiskTier::Read,
        RiskTier::Service,
        RiskTier::Adapt,
        RiskTier::Code,
        RiskTier::Security,
        RiskTier::Flash,
    ]
    .into_iter()
    .map(|t| TierView {
        id: t,
        label: t.label(),
        explain: t.explain(),
        confirmations: t.confirmations(),
    })
    .collect();
    let functions = catalog::FUNCTIONS
        .iter()
        .map(|f| FunctionView {
            function: *f,
            tier_label: f.tier.label(),
            confirmations: f.tier.confirmations(),
            requirements: f.requires.iter().copied().map(Into::into).collect(),
            verified_on_car: job_verified(f.id),
        })
        .collect();
    Catalog {
        tiers,
        functions,
        pids: obd2::PIDS,
        modules: bmw_e70::MODULES,
    }
}

#[tauri::command]
fn list_devices() -> Vec<DiscoveredDevice> {
    vec![
        DiscoveredDevice {
            id: "sim-idle".into(),
            name: "Demo X5 · engine idling".into(),
            detail: "Simulated 2008 E70 X5 in Park with the engine running. Fake data.".into(),
            transport: TransportKind::Simulator { engine_running: true },
            available: true,
        },
        DiscoveredDevice {
            id: "sim-off".into(),
            name: "Demo X5 · ignition on".into(),
            detail: "Simulated 2008 E70 X5 with ignition on and engine off, ready for service functions. Fake data.".into(),
            transport: TransportKind::Simulator { engine_running: false },
            available: true,
        },
        DiscoveredDevice {
            id: "veepeak-ble".into(),
            name: "Veepeak OBDCheck BLE".into(),
            detail: "Bluetooth LE adapter. Live connection arrives in the next step.".into(),
            transport: TransportKind::Ble { peripheral_id: String::new() },
            available: false,
        },
    ]
}

#[derive(Serialize)]
struct ConnectReport {
    device: DiscoveredDevice,
    adapter: AdapterInfo,
    vin: Option<String>,
    battery_v: Option<f32>,
    mil: Option<MilStatus>,
    supported_pids: Vec<u8>,
    simulated: bool,
    /// The adapter conversation during setup, for the connection log.
    transcript: Vec<String>,
}

#[tauri::command]
async fn connect(state: State<'_, AppState>, id: String) -> CmdResult<ConnectReport> {
    let device = list_devices()
        .into_iter()
        .find(|d| d.id == id)
        .ok_or_else(|| CmdError::new("unknown", "Unknown adapter"))?;
    let link: Box<dyn HardwareTransport + Send> = match &device.transport {
        TransportKind::Simulator { engine_running } => {
            Box::new(ElmSimulator::new(if *engine_running {
                SimEngine::Idling
            } else {
                SimEngine::IgnitionOn
            }))
        }
        _ => {
            return Err(CmdError::new(
                "unavailable",
                "This adapter is not supported yet.",
            ))
        }
    };
    let mut inner = state.0.lock().await;
    let mut client = ElmClient::new(link);
    let adapter = client.connect().await?;
    let vin = client.read_vin().await.ok();
    let battery_v = client.battery_voltage().await.ok();
    let mil = match client.obd(&[0x01, 0x01]).await {
        Ok(replies) => replies
            .first()
            .and_then(|(_, p)| obd2::decode_mil_status(p).ok()),
        Err(_) => None,
    };
    let supported_pids = client.supported_pids().await.unwrap_or_default();
    let transcript = client
        .link_mut()
        .simulator()
        .map(|s| s.log.clone())
        .unwrap_or_default();
    let simulated = matches!(device.transport, TransportKind::Simulator { .. });
    inner.client = Some(client);
    inner.device = Some(device.clone());
    inner.charger_confirmed = false;
    let target = vin.clone().unwrap_or_else(|| "vehicle".into());
    inner.record(
        "Connect",
        &target,
        RiskTier::Read,
        true,
        format!("Connected via {}", device.name),
    );
    Ok(ConnectReport {
        device,
        adapter,
        vin,
        battery_v,
        mil,
        supported_pids,
        simulated,
        transcript,
    })
}

#[tauri::command]
async fn disconnect(state: State<'_, AppState>) -> CmdResult<()> {
    let mut inner = state.0.lock().await;
    if let Some(mut c) = inner.client.take() {
        let _ = c.disconnect().await;
        inner.record(
            "Disconnect",
            "vehicle",
            RiskTier::Read,
            true,
            "Disconnected",
        );
    }
    inner.device = None;
    Ok(())
}

// ---- commands: live data and state -------------------------------------

#[derive(Serialize)]
struct LiveValue {
    pid: u8,
    value: f64,
}

#[tauri::command]
async fn read_live(state: State<'_, AppState>, pids: Vec<u8>) -> CmdResult<Vec<LiveValue>> {
    let mut inner = state.0.lock().await;
    let c = inner.client()?;
    let mut out = Vec::with_capacity(pids.len());
    for pid in pids {
        if let Ok(value) = c.read_pid(pid).await {
            out.push(LiveValue { pid, value });
        }
    }
    Ok(out)
}

#[derive(Serialize)]
struct FunctionReadiness {
    id: &'static str,
    unmet: Vec<RequirementView>,
}

#[derive(Serialize)]
struct StateReport {
    state: VehicleState,
    engine_running: Option<bool>,
    functions: Vec<FunctionReadiness>,
}

#[tauri::command]
async fn vehicle_state(state: State<'_, AppState>) -> CmdResult<StateReport> {
    let mut inner = state.0.lock().await;
    let simulated = inner.simulated();
    let mut base = inner.vehicle_state(None).await?;
    let functions = catalog::FUNCTIONS
        .iter()
        .map(|f| {
            base.job_verified = simulated || job_verified(f.id);
            FunctionReadiness {
                id: f.id,
                unmet: f.unmet(&base).into_iter().map(Into::into).collect(),
            }
        })
        .collect();
    base.job_verified = simulated;
    let engine_running = base.rpm.map(|r| r > 300.0);
    Ok(StateReport {
        state: base,
        engine_running,
        functions,
    })
}

#[tauri::command]
async fn set_charger_confirmed(state: State<'_, AppState>, on: bool) -> CmdResult<()> {
    let mut inner = state.0.lock().await;
    inner.charger_confirmed = on;
    if let Some(sim) = inner.client.as_mut().and_then(|c| c.link_mut().simulator()) {
        sim.set_charger(on);
    }
    Ok(())
}

/// Demo only: start or stop the simulated engine.
#[tauri::command]
async fn sim_set_engine(state: State<'_, AppState>, running: bool) -> CmdResult<()> {
    let mut inner = state.0.lock().await;
    let sim = inner.client()?.link_mut().simulator().ok_or_else(|| {
        CmdError::new(
            "unavailable",
            "Only the demo car can be started from the app.",
        )
    })?;
    sim.set_engine(if running {
        SimEngine::Idling
    } else {
        SimEngine::IgnitionOn
    });
    let action = if running {
        "Demo: start engine"
    } else {
        "Demo: stop engine"
    };
    inner.record(action, "simulator", RiskTier::Read, true, "Done");
    Ok(())
}

// ---- commands: fault codes ---------------------------------------------

#[derive(Serialize)]
struct ObdCodes {
    stored: Vec<Dtc>,
    pending: Vec<Dtc>,
    permanent: Vec<Dtc>,
    mil: Option<MilStatus>,
}

#[tauri::command]
async fn read_obd_codes(state: State<'_, AppState>) -> CmdResult<ObdCodes> {
    let mut inner = state.0.lock().await;
    let c = inner.client()?;
    let stored = c.read_obd_dtcs(Service::StoredDtcs).await?;
    let pending = c
        .read_obd_dtcs(Service::PendingDtcs)
        .await
        .unwrap_or_default();
    let permanent = c
        .read_obd_dtcs(Service::PermanentDtcs)
        .await
        .unwrap_or_default();
    let mil = match c.obd(&[0x01, 0x01]).await {
        Ok(r) => r.first().and_then(|(_, p)| obd2::decode_mil_status(p).ok()),
        Err(_) => None,
    };
    let n = stored.len() + pending.len() + permanent.len();
    inner.record(
        "Read fault codes",
        "Engine (OBD-II)",
        RiskTier::Read,
        true,
        format!("{n} codes"),
    );
    Ok(ObdCodes {
        stored,
        pending,
        permanent,
        mil,
    })
}

#[tauri::command]
async fn clear_obd_codes(state: State<'_, AppState>) -> CmdResult<Vec<Dtc>> {
    let mut inner = state.0.lock().await;
    inner.gate("clear_codes", "Engine (OBD-II)").await?;
    let c = inner.client()?;
    // Save what we are about to erase.
    let mut saved = c.read_obd_dtcs(Service::StoredDtcs).await?;
    saved.extend(
        c.read_obd_dtcs(Service::PendingDtcs)
            .await
            .unwrap_or_default(),
    );
    c.clear_obd_dtcs().await?;
    let codes: Vec<_> = saved.iter().map(|d| d.code.clone()).collect();
    inner.record(
        "Clear fault codes",
        "Engine (OBD-II)",
        RiskTier::Service,
        true,
        format!(
            "Cleared {} (saved in this log)",
            if codes.is_empty() {
                "nothing".into()
            } else {
                codes.join(", ")
            }
        ),
    );
    Ok(saved)
}

#[derive(Serialize)]
struct ModuleFault {
    code: String,
    status: u8,
    description: Option<&'static str>,
}

#[derive(Serialize)]
struct ModuleScan {
    module: Module,
    present: bool,
    faults: Option<usize>,
}

#[tauri::command]
async fn scan_modules(state: State<'_, AppState>) -> CmdResult<Vec<ModuleScan>> {
    let mut inner = state.0.lock().await;
    let c = inner.client()?;
    let mut out = Vec::new();
    for m in bmw_e70::MODULES {
        let present = c.probe(m).await.unwrap_or(false);
        let faults = if present {
            c.read_module_dtcs(m).await.ok().map(|d| d.len())
        } else {
            None
        };
        out.push(ModuleScan {
            module: *m,
            present,
            faults,
        });
    }
    let found = out.iter().filter(|m| m.present).count();
    inner.record(
        "Module scan",
        "All modules",
        RiskTier::Read,
        true,
        format!("{found} modules answered"),
    );
    Ok(out)
}

#[derive(Serialize)]
struct ModuleDetail {
    module: Module,
    identification: Option<String>,
    faults: Vec<ModuleFault>,
}

fn module_by_address(address: u8) -> CmdResult<&'static Module> {
    bmw_e70::module_at(address).ok_or_else(|| CmdError::new("unknown", "Unknown module"))
}

#[tauri::command]
async fn read_module(state: State<'_, AppState>, address: u8) -> CmdResult<ModuleDetail> {
    let module = module_by_address(address)?;
    let mut inner = state.0.lock().await;
    let simulated = inner.simulated();
    let c = inner.client()?;
    let identification = c.read_module_identification(module).await.ok();
    let faults = c
        .read_module_dtcs(module)
        .await?
        .into_iter()
        .map(|d| ModuleFault {
            description: if simulated {
                sim::sim_fault_text(address, &d.code)
            } else {
                None
            },
            code: d.code,
            status: d.status,
        })
        .collect::<Vec<_>>();
    inner.record(
        "Read fault memory",
        module.name,
        RiskTier::Read,
        true,
        format!("{} faults", faults.len()),
    );
    Ok(ModuleDetail {
        module: *module,
        identification,
        faults,
    })
}

#[tauri::command]
async fn clear_module_codes(state: State<'_, AppState>, address: u8) -> CmdResult<()> {
    let module = module_by_address(address)?;
    let mut inner = state.0.lock().await;
    inner.gate("clear_codes", module.name).await?;
    let c = inner.client()?;
    let saved = c.read_module_dtcs(module).await?;
    c.bmw(
        module,
        &protocol_core::diag::request::clear_dtcs(module.protocol()),
    )
    .await?;
    let codes: Vec<_> = saved.iter().map(|d| d.code.clone()).collect();
    inner.record(
        "Clear fault memory",
        module.name,
        RiskTier::Service,
        true,
        format!(
            "Cleared {}",
            if codes.is_empty() {
                "nothing".into()
            } else {
                codes.join(", ")
            }
        ),
    );
    Ok(())
}

// ---- commands: special functions ---------------------------------------

#[derive(Serialize)]
struct FunctionRun {
    simulated: bool,
    steps: Vec<String>,
}

#[tauri::command]
async fn run_function(state: State<'_, AppState>, id: String) -> CmdResult<FunctionRun> {
    let mut inner = state.0.lock().await;
    let simulated = inner.simulated();
    let f = inner.gate(&id, "vehicle").await?;
    if !simulated {
        // Real-car payloads are not verified yet; gate() already refused,
        // but be explicit in case the catalog changes.
        return Err(CmdError::new(
            "unverified",
            "This routine is not verified for a real car yet.",
        ));
    }
    let mut steps = vec!["Safety checks passed".to_string()];
    if f.tier >= RiskTier::Code {
        steps.push("Backed up module data".into());
    }
    steps.extend(
        match f.id {
            "coolant_bleed" => vec![
                "Electric water pump: bleed cycle started",
                "Pump at full speed, auxiliary valve open",
                "Pump cycling to move trapped air",
                "Bleed cycle finished. Top up coolant to the MAX mark when cool.",
            ],
            "battery_registration" => vec![
                "Read current battery history",
                "Registered battery replacement",
                "Charging strategy reset for a new battery",
            ],
            "dme_adaptation_reset" => vec![
                "Fuel trim adaptations cleared",
                "Idle and throttle adaptations cleared",
                "Drive normally; the engine relearns over the next 20 to 50 km",
            ],
            "egs_adaptation_reset" => vec![
                "Clutch pressure adaptations cleared",
                "Shift adaptations cleared",
                "Shifts may feel firm until relearned over a few drives",
            ],
            "actuator_test" => vec![
                "Radiator fan on at 30%",
                "Radiator fan off",
                "Test finished; control returned to the DME",
            ],
            "coding" => vec![
                "Read current coding",
                "Wrote new values",
                "Verified by reading back",
            ],
            "key_programming" => vec![
                "Verified VIN match between CAS and DME",
                "Read key slots: 2 in use, 8 free",
                "Programmed key into slot 3",
                "Test: hold the new key to the steering column and press Start",
            ],
            "flash" => vec![
                "Saved original software",
                "Wrote new software",
                "Verified checksum",
            ],
            _ => vec!["Done"],
        }
        .into_iter()
        .map(String::from),
    );
    inner.record(f.name, "vehicle", f.tier, true, "Completed (demo car)");
    Ok(FunctionRun { simulated, steps })
}

#[tauri::command]
async fn activity_log(state: State<'_, AppState>) -> CmdResult<Vec<serde_json::Value>> {
    let inner = state.0.lock().await;
    Ok(inner
        .log
        .iter()
        .rev()
        .map(|e| serde_json::to_value(e).unwrap_or_default())
        .collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            catalog,
            list_devices,
            connect,
            disconnect,
            read_live,
            vehicle_state,
            set_charger_confirmed,
            sim_set_engine,
            read_obd_codes,
            clear_obd_codes,
            scan_modules,
            read_module,
            clear_module_codes,
            run_function,
            activity_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ApexDiag");
}
