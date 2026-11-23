//! Tauri shell: exposes the Rust diagnostic core to the React UI.
//!
//! Every command that changes something in the car goes through
//! [`Inner::gate`], which reads the car's live state and refuses to run when
//! any safety requirement is not proven. Every attempt, allowed or blocked,
//! is written to the activity log.

mod timemachine;

use protocol_core::catalog::{self, Function, Requirement, RiskTier, VehicleState};
use protocol_core::obd2::{self, Dtc, MilStatus, PidInfo, Readiness, Service};
use protocol_core::snapshot::{
    self, ModuleDiff, RecordKind, RecordValue, Snapshot, SnapshotFault, SnapshotKind,
};
use protocol_core::vehicle::{self, JobStatus, Layout, Module, Pack, System};
use protocol_core::vin::{self, VinInfo};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};
use timemachine::SnapshotStore;
use tokio::sync::Mutex;
use transport::elm::{AdapterInfo, ClientError, ElmClient, FreezeFrame};
use transport::sim::{self, ElmSimulator, SimCar, SimEngine};
use transport::{DiscoveredDevice, HardwareTransport, TransportKind};

type Client = ElmClient<Box<dyn HardwareTransport + Send>>;

#[derive(Default)]
struct Inner {
    client: Option<Client>,
    device: Option<DiscoveredDevice>,
    vin: Option<String>,
    /// The vehicle pack for the connected car (generic OBD-II if unknown).
    pack: Option<&'static Pack>,
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
    packs: Vec<&'static Pack>,
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
    fn pack(&self) -> &'static Pack {
        self.pack.unwrap_or(&vehicle::GENERIC)
    }

    fn module(&self, address: u8) -> CmdResult<&'static Module> {
        self.pack()
            .module(address)
            .ok_or_else(|| CmdError::new("unknown", "This car has no such module."))
    }

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
        let pack = self.pack();
        let c = self.client()?;
        let speed = c.read_pid(0x0D).await.ok();
        let rpm = c.read_pid(0x0C).await.ok();
        let battery = c.battery_voltage().await.ok().map(|v| v as f64);
        let job_verified = match function_id {
            _ if simulated => true,
            Some(id) => job_verified(pack, id),
            None => false,
        };
        Ok(VehicleState {
            speed_kmh: speed,
            rpm,
            battery_v: battery,
            charger_confirmed: charger,
            wired_adapter: false,
            // The Time Machine backs up the target module right before every
            // write, and gate() refuses the write if that backup fails.
            backup_taken: true,
            job_verified,
        })
    }

    /// Refuses (and logs) when any requirement for `function_id` is unmet.
    /// When everything else is met, snapshots each module in `backup` to the
    /// Time Machine; if a required backup fails, the write is refused.
    async fn gate(
        &mut self,
        function_id: &str,
        target: &str,
        store: &SnapshotStore,
        backup: &[&'static Module],
    ) -> CmdResult<Gated> {
        let f = catalog::function(function_id)
            .ok_or_else(|| CmdError::new("unknown", format!("Unknown function {function_id}")))?;
        let state = self.vehicle_state(Some(function_id)).await?;
        let mut unmet = f.unmet(&state);
        let mut snapshots = Vec::new();
        if unmet.is_empty() && f.tier > RiskTier::Read {
            let needs_backup = f.requires.contains(&Requirement::BackupTaken);
            for m in backup {
                let reason = format!("Before {}", f.name.to_lowercase());
                match self.capture(m, SnapshotKind::Automatic, reason).await {
                    Ok(s) if s.is_complete() || !needs_backup => match store.save(&s) {
                        Ok(()) => snapshots.push(s),
                        Err(_) if needs_backup => {
                            unmet.push(Requirement::BackupTaken);
                            break;
                        }
                        Err(_) => {}
                    },
                    _ if needs_backup => {
                        unmet.push(Requirement::BackupTaken);
                        break;
                    }
                    _ => {}
                }
            }
        }
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
        Ok(Gated {
            function: f,
            snapshots,
        })
    }

    /// Reads one module's identification, fault memory and records.
    async fn capture(
        &mut self,
        module: &'static Module,
        kind: SnapshotKind,
        reason: String,
    ) -> CmdResult<Snapshot> {
        let simulated = self.simulated();
        let vin = self.vin.clone().unwrap_or_else(|| "UNKNOWN".into());
        let pack = self.pack();
        let c = self.client()?;
        let identification = c.read_module_identification(module).await.ok();
        let faults = c
            .read_module_dtcs(module)
            .await?
            .into_iter()
            .map(|d| SnapshotFault {
                code: d.code,
                status: d.status,
            })
            .collect();
        let mut records = Vec::new();
        let mut missing = Vec::new();
        // Unverified identifiers are only ever sent to the demo car.
        for d in pack
            .records_for(module.address)
            .filter(|d| simulated || d.status == JobStatus::Verified)
        {
            match c.read_record(module, d.id).await {
                Ok(raw) if raw.len() == d.len() => records.push(RecordValue { id: d.id, raw }),
                _ => missing.push(d.id),
            }
        }
        let at = now_ms() as u64;
        Ok(Snapshot {
            id: Snapshot::make_id(at, module.address),
            vin,
            module_address: module.address,
            module_name: module.name.into(),
            taken_at_ms: at,
            kind,
            reason,
            simulated,
            identification,
            records,
            faults,
            missing,
        })
    }

    fn vin(&self) -> CmdResult<String> {
        self.vin.clone().ok_or_else(|| {
            CmdError::new(
                "no_vin",
                "The car's VIN could not be read, so snapshots can't be filed.",
            )
        })
    }
}

/// A gate that passed, with the backups it took.
struct Gated {
    function: &'static Function,
    snapshots: Vec<Snapshot>,
}

impl Gated {
    fn backup_step(&self) -> Option<String> {
        match self.snapshots.as_slice() {
            [] => None,
            s => Some(format!(
                "Backed up {} to the Time Machine",
                s.iter()
                    .map(|s| s.module_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

/// The engine module of a pack, if it has one.
fn engine_module(pack: &'static Pack) -> Option<&'static Module> {
    pack.modules.iter().find(|m| m.system == System::Powertrain)
}

/// The module that holds keys and the immobilizer.
fn key_module(pack: &'static Pack) -> Option<&'static Module> {
    pack.modules
        .iter()
        .find(|m| ["CAS", "EZS", "SMART KEY", "KEYLESS"].contains(&m.name))
}

/// Modules a special function writes to, which get backed up first.
fn function_targets(pack: &'static Pack, function_id: &str) -> Vec<&'static Module> {
    let mut out: Vec<&'static Module> = pack
        .jobs
        .iter()
        .filter(|j| j.function_id == function_id)
        .filter_map(|j| pack.module(j.module))
        .collect();
    out.dedup_by_key(|m| m.address);
    if out.is_empty() {
        out.extend(match function_id {
            "actuator_test" | "flash" => engine_module(pack),
            "coding" => pack
                .records
                .iter()
                .find(|r| r.kind == RecordKind::Coding)
                .and_then(|r| pack.module(r.module)),
            "key_programming" => key_module(pack),
            _ => None,
        });
    }
    out
}

/// True when every job behind a function has verified request bytes.
fn job_verified(pack: &Pack, function_id: &str) -> bool {
    match catalog::function(function_id).map(|f| f.tier) {
        Some(RiskTier::Read) => true,
        // Clearing codes uses standard services, not brand-specific jobs.
        _ if function_id == "clear_codes" => true,
        _ => {
            let jobs: Vec<_> = pack
                .jobs
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
            // Verified for a real car on at least one pack.
            verified_on_car: vehicle::packs().iter().any(|p| job_verified(p, f.id)),
        })
        .collect();
    Catalog {
        tiers,
        functions,
        pids: obd2::PIDS,
        packs: vehicle::packs().to_vec(),
    }
}

const SIM_PREFIX: &str = "sim:";

#[tauri::command]
fn list_devices() -> Vec<DiscoveredDevice> {
    let mut out: Vec<DiscoveredDevice> = sim::SIM_CARS
        .iter()
        .map(|car| DiscoveredDevice {
            id: format!("{SIM_PREFIX}{}", car.id),
            name: format!("Demo {}", car.label),
            detail: format!("{} Practice car, fake data.", car.story),
            transport: TransportKind::Simulator {
                engine_running: true,
            },
            available: true,
        })
        .collect();
    out.push(DiscoveredDevice {
        id: "veepeak-ble".into(),
        name: "Veepeak OBDCheck BLE".into(),
        detail: "Bluetooth LE adapter. Live connection arrives in the next step.".into(),
        transport: TransportKind::Ble {
            peripheral_id: String::new(),
        },
        available: false,
    });
    out
}

fn sim_car_for(device_id: &str) -> Option<&'static SimCar> {
    device_id.strip_prefix(SIM_PREFIX).and_then(sim::sim_car)
}

/// What the app knows about the car it is talking to.
#[derive(Serialize)]
struct VehicleView {
    vin_info: Option<VinInfo>,
    pack: &'static Pack,
    /// Modules the scan will ask for: the whole pack on the demo car or a
    /// documented layout, only the legislated OBD-II IDs otherwise.
    scan_addresses: Vec<u8>,
    demo_story: Option<&'static str>,
}

impl VehicleView {
    fn new(pack: &'static Pack, vin: Option<&str>, car: Option<&'static SimCar>) -> Self {
        let scan_addresses = if car.is_some() || pack.layout == Layout::Documented {
            pack.modules.iter().map(|m| m.address).collect()
        } else {
            pack.scannable_on_real_car().map(|m| m.address).collect()
        };
        Self {
            vin_info: vin.map(|v| vin::decode(v, current_year())),
            pack,
            scan_addresses,
            demo_story: car.map(|c| c.story),
        }
    }
}

fn current_year() -> u16 {
    // Days since 1970 / 365.2425, good enough for model-year decoding.
    (1970 + now_ms() / 1000 / 86_400 * 10_000 / 3_652_425) as u16
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
    vehicle: VehicleView,
    /// The adapter conversation during setup, for the connection log.
    transcript: Vec<String>,
}

#[tauri::command]
async fn connect(
    state: State<'_, AppState>,
    store: State<'_, SnapshotStore>,
    id: String,
) -> CmdResult<ConnectReport> {
    let device = list_devices()
        .into_iter()
        .find(|d| d.id == id)
        .ok_or_else(|| CmdError::new("unknown", "Unknown adapter"))?;
    let car = sim_car_for(&device.id);
    let link: Box<dyn HardwareTransport + Send> = match (&device.transport, car) {
        (TransportKind::Simulator { engine_running }, Some(car)) => {
            Box::new(ElmSimulator::with_car(
                car,
                if *engine_running {
                    SimEngine::Idling
                } else {
                    SimEngine::IgnitionOn
                },
            ))
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
    // The demo car's pack, or the pack for the make the VIN names.
    let pack: &'static Pack = match (car, &vin) {
        (Some(car), _) => car.pack,
        (None, Some(v)) => vehicle::pack_for_make(vin::decode(v, current_year()).make),
        (None, None) => &vehicle::GENERIC,
    };
    if let (Some(car), Some(v)) = (car, &vin) {
        let now = now_ms() as u64;
        let _ = if car.pack.id == "bmw_e70" {
            timemachine::seed_demo_history(&store, v, now)
        } else {
            let current = client
                .link_mut()
                .simulator()
                .map(|s| s.records())
                .unwrap_or_default();
            timemachine::seed_generic_history(&store, v, car.pack, &current, now)
        };
    }
    let vehicle = VehicleView::new(pack, vin.as_deref(), car);
    inner.client = Some(client);
    inner.vin = vin.clone();
    inner.pack = Some(pack);
    inner.device = Some(device.clone());
    inner.charger_confirmed = false;
    let target = vin.clone().unwrap_or_else(|| "vehicle".into());
    inner.record(
        "Connect",
        &target,
        RiskTier::Read,
        true,
        format!(
            "Connected via {} ({} {})",
            device.name, pack.brand, pack.model
        ),
    );
    Ok(ConnectReport {
        device,
        adapter,
        vin,
        battery_v,
        mil,
        supported_pids,
        simulated,
        vehicle,
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
    inner.vin = None;
    inner.pack = None;
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
    let pack = inner.pack();
    let mut base = inner.vehicle_state(None).await?;
    let functions = catalog::FUNCTIONS
        .iter()
        .map(|f| {
            base.job_verified = simulated || job_verified(pack, f.id);
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
async fn clear_obd_codes(
    state: State<'_, AppState>,
    store: State<'_, SnapshotStore>,
) -> CmdResult<Vec<Dtc>> {
    let mut inner = state.0.lock().await;
    let backup: Vec<&'static Module> = engine_module(inner.pack()).into_iter().collect();
    inner
        .gate("clear_codes", "Engine (OBD-II)", &store, &backup)
        .await?;
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
    codes: Vec<ModuleFault>,
    /// How long the module took to answer, for the Quick Test.
    elapsed_ms: u64,
}

/// Plain-language text for a module fault: the demo car's own text, else
/// the generic SAE meaning of the code's first five characters.
fn fault_text(simulated: bool, pack: &Pack, address: u8, code: &str) -> Option<&'static str> {
    simulated
        .then(|| sim::sim_fault_text(pack.id, address, code))
        .flatten()
        .or_else(|| code.get(..5).and_then(obd2::describe_dtc))
}

async fn scan_one(
    c: &mut Client,
    simulated: bool,
    pack: &'static Pack,
    m: &'static Module,
) -> ModuleScan {
    let started = std::time::Instant::now();
    let present = c.probe(m).await.unwrap_or(false);
    let dtcs = if present {
        c.read_module_dtcs(m).await.ok()
    } else {
        None
    };
    let codes = dtcs
        .iter()
        .flatten()
        .map(|d| ModuleFault {
            description: fault_text(simulated, pack, m.address, &d.code),
            code: d.code.clone(),
            status: d.status,
        })
        .collect();
    ModuleScan {
        module: *m,
        present,
        faults: dtcs.map(|d| d.len()),
        codes,
        elapsed_ms: started.elapsed().as_millis() as u64,
    }
}

/// Which modules may be scanned on this car (see [`VehicleView`]).
fn scan_list(inner: &Inner) -> Vec<&'static Module> {
    let pack = inner.pack();
    if inner.simulated() || pack.layout == Layout::Documented {
        pack.modules.iter().collect()
    } else {
        pack.scannable_on_real_car().collect()
    }
}

/// Scans one module: used by the Quick Test to show progress per module.
#[tauri::command]
async fn scan_module(state: State<'_, AppState>, address: u8) -> CmdResult<ModuleScan> {
    let mut inner = state.0.lock().await;
    let module = inner.module(address)?;
    if !scan_list(&inner).iter().any(|m| m.address == address) {
        return Err(CmdError::new(
            "unverified",
            "This module's address is not confirmed for a real car yet, so it is not scanned.",
        ));
    }
    let simulated = inner.simulated();
    let pack = inner.pack();
    let c = inner.client()?;
    Ok(scan_one(c, simulated, pack, module).await)
}

/// Records a finished Quick Test in the activity log.
#[tauri::command]
async fn log_quick_test(state: State<'_, AppState>, found: usize, faults: usize) -> CmdResult<()> {
    let mut inner = state.0.lock().await;
    inner.record(
        "Quick Test",
        "All modules",
        RiskTier::Read,
        true,
        format!("{found} modules answered, {faults} faults"),
    );
    Ok(())
}

#[tauri::command]
async fn scan_modules(state: State<'_, AppState>) -> CmdResult<Vec<ModuleScan>> {
    let mut inner = state.0.lock().await;
    let list = scan_list(&inner);
    let simulated = inner.simulated();
    let pack = inner.pack();
    let c = inner.client()?;
    let mut out = Vec::new();
    for m in list {
        out.push(scan_one(c, simulated, pack, m).await);
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

#[tauri::command]
async fn read_module(state: State<'_, AppState>, address: u8) -> CmdResult<ModuleDetail> {
    let mut inner = state.0.lock().await;
    let module = inner.module(address)?;
    let simulated = inner.simulated();
    let pack = inner.pack();
    let c = inner.client()?;
    let identification = c.read_module_identification(module).await.ok();
    let faults = c
        .read_module_dtcs(module)
        .await?
        .into_iter()
        .map(|d| ModuleFault {
            description: fault_text(simulated, pack, address, &d.code),
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
async fn clear_module_codes(
    state: State<'_, AppState>,
    store: State<'_, SnapshotStore>,
    address: u8,
) -> CmdResult<()> {
    let mut inner = state.0.lock().await;
    let module = inner.module(address)?;
    inner
        .gate("clear_codes", module.name, &store, &[module])
        .await?;
    let c = inner.client()?;
    let saved = c.read_module_dtcs(module).await?;
    c.clear_module_dtcs(module).await?;
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
                format!("{} (kept in the Time Machine)", codes.join(", "))
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
async fn run_function(
    state: State<'_, AppState>,
    store: State<'_, SnapshotStore>,
    id: String,
) -> CmdResult<FunctionRun> {
    if id == "snapshot_restore" {
        return Err(CmdError::new(
            "unavailable",
            "Restore coding from the Time Machine, where you pick the snapshot.",
        ));
    }
    let mut inner = state.0.lock().await;
    let simulated = inner.simulated();
    let pack = inner.pack();
    if !pack.offers(&id) {
        return Err(CmdError::new(
            "unavailable",
            format!(
                "This function is not available for the {} {}.",
                pack.brand, pack.model
            ),
        ));
    }
    let targets = function_targets(pack, &id);
    let gated = inner.gate(&id, "vehicle", &store, &targets).await?;
    let f = gated.function;
    if !simulated {
        // Real-car payloads are not verified yet; gate() already refused,
        // but be explicit in case the catalog changes.
        return Err(CmdError::new(
            "unverified",
            "This routine is not verified for a real car yet.",
        ));
    }
    let mut steps = vec!["Safety checks passed".to_string()];
    steps.extend(gated.backup_step());
    // What the routine changes inside the demo car.
    if let Some(sim) = inner.client()?.link_mut().simulator() {
        if f.id.ends_with("_adaptation_reset") {
            for m in &targets {
                sim.reset_adaptations(m.address);
            }
        }
    }
    let engine = engine_module(pack)
        .map(|m| m.name)
        .unwrap_or("engine module");
    let keys = key_module(pack).map(|m| m.name).unwrap_or("immobilizer");
    let actuator_done = format!("Test finished; control returned to the {engine}");
    let key_check = format!("Verified VIN match between {keys} and {engine}");
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
                actuator_done.as_str(),
            ],
            "coding" => vec![
                "Read current coding",
                "Wrote new values",
                "Verified by reading back",
            ],
            "key_programming" => vec![
                key_check.as_str(),
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
        .map(|s| s.to_string()),
    );
    inner.record(f.name, "vehicle", f.tier, true, "Completed (demo car)");
    Ok(FunctionRun { simulated, steps })
}

// ---- commands: Time Machine -------------------------------------------

/// Every saved snapshot for the connected car, newest first.
#[tauri::command]
async fn list_snapshots(
    state: State<'_, AppState>,
    store: State<'_, SnapshotStore>,
) -> CmdResult<Vec<Snapshot>> {
    let inner = state.0.lock().await;
    let vin = inner.vin()?;
    Ok(store.list(&vin))
}

/// Reads a module now and saves it as a manual snapshot.
#[tauri::command]
async fn take_snapshot(
    state: State<'_, AppState>,
    store: State<'_, SnapshotStore>,
    address: u8,
) -> CmdResult<Snapshot> {
    let mut inner = state.0.lock().await;
    let module = inner.module(address)?;
    inner.vin()?;
    let s = inner
        .capture(module, SnapshotKind::Manual, "Taken by hand".into())
        .await?;
    store
        .save(&s)
        .map_err(|e| CmdError::new("storage", format!("Could not save the snapshot: {e}")))?;
    inner.record(
        "Take snapshot",
        module.name,
        RiskTier::Read,
        true,
        format!("{} records, {} faults", s.records.len(), s.faults.len()),
    );
    Ok(s)
}

/// `"live"` means: read the module right now (not saved).
const LIVE: &str = "live";

#[derive(Serialize)]
struct Comparison {
    from: Snapshot,
    to: Snapshot,
    diff: ModuleDiff,
}

async fn load_or_read(
    inner: &mut Inner,
    store: &SnapshotStore,
    module: &'static Module,
    id: &str,
) -> CmdResult<Snapshot> {
    if id == LIVE {
        let mut s = inner
            .capture(module, SnapshotKind::Manual, "Now".into())
            .await?;
        s.id = LIVE.into();
        return Ok(s);
    }
    let vin = inner.vin()?;
    store
        .get(&vin, id)
        .filter(|s| s.module_address == module.address)
        .ok_or_else(|| CmdError::new("not_found", "That snapshot no longer exists."))
}

#[tauri::command]
async fn compare_snapshots(
    state: State<'_, AppState>,
    store: State<'_, SnapshotStore>,
    address: u8,
    from: String,
    to: String,
) -> CmdResult<Comparison> {
    let mut inner = state.0.lock().await;
    let module = inner.module(address)?;
    let from = load_or_read(&mut inner, &store, module, &from).await?;
    let to = load_or_read(&mut inner, &store, module, &to).await?;
    let diff = snapshot::diff(&from, &to, inner.pack().records);
    Ok(Comparison { from, to, diff })
}

#[derive(Serialize)]
struct RestoreReport {
    steps: Vec<String>,
    written: usize,
    /// Snapshot taken just before the restore, to undo it.
    undo_id: Option<String>,
}

/// Writes a module's coding back to a saved snapshot, behind the full
/// coding gate. Each write is read back to confirm it.
#[tauri::command]
async fn restore_snapshot(
    state: State<'_, AppState>,
    store: State<'_, SnapshotStore>,
    address: u8,
    id: String,
) -> CmdResult<RestoreReport> {
    let mut inner = state.0.lock().await;
    let module = inner.module(address)?;
    let vin = inner.vin()?;
    let target = store
        .get(&vin, &id)
        .filter(|s| s.module_address == address)
        .ok_or_else(|| CmdError::new("not_found", "That snapshot no longer exists."))?;
    let gated = inner
        .gate("snapshot_restore", module.name, &store, &[module])
        .await?;
    let before = gated
        .snapshots
        .first()
        .cloned()
        .ok_or_else(|| CmdError::new("backup", "The safety backup was not taken."))?;
    let plan = snapshot::restore_plan(&before, &target, inner.pack().records);
    let mut steps = vec!["Safety checks passed".to_string()];
    steps.extend(gated.backup_step());
    let undo_id = Some(before.id.clone());
    if plan.is_empty() {
        steps.push("Nothing to write: the coding already matches this snapshot".into());
        inner.record(
            "Restore module coding",
            module.name,
            RiskTier::Code,
            true,
            "Already matched, nothing written",
        );
        return Ok(RestoreReport {
            steps,
            written: 0,
            undo_id,
        });
    }
    let c = inner.client()?;
    let mut done = 0;
    let mut failure = None;
    for w in &plan {
        let result = async {
            c.write_record(module, w.id, &w.raw).await?;
            let back = c.read_record(module, w.id).await?;
            Ok::<bool, ClientError>(back == w.raw)
        }
        .await;
        match result {
            Ok(true) => {
                done += 1;
                steps.push(format!("{}: {} → {} (read back OK)", w.name, w.from, w.to));
            }
            Ok(false) => {
                failure = Some(format!("{} did not read back as written", w.name));
                break;
            }
            Err(e) => {
                failure = Some(format!("{}: {}", w.name, CmdError::from(e).message));
                break;
            }
        }
    }
    if let Some(why) = failure {
        inner.record(
            "Restore module coding",
            module.name,
            RiskTier::Code,
            false,
            format!("Stopped after {done} of {} writes: {why}", plan.len()),
        );
        return Err(CmdError::new(
            "restore_failed",
            format!(
                "Restore stopped after {done} of {} settings: {why}. The module's earlier coding is saved in the Time Machine as \"{}\".",
                plan.len(),
                before.reason
            ),
        ));
    }
    steps.push(format!(
        "Restored {done} setting{} to {}",
        if done == 1 { "" } else { "s" },
        target.reason.to_lowercase()
    ));
    inner.record(
        "Restore module coding",
        module.name,
        RiskTier::Code,
        true,
        format!("Wrote {done} settings from snapshot {}", target.id),
    );
    Ok(RestoreReport {
        steps,
        written: done,
        undo_id,
    })
}

// ---- commands: emissions readiness ------------------------------------

#[derive(Serialize)]
struct EmissionsReport {
    readiness: Option<Readiness>,
    freeze_frame: Option<FreezeFrame>,
    /// Distance driven since codes were last cleared (PID 31), in km.
    km_since_clear: Option<f64>,
    /// Warm-ups since codes were last cleared (PID 30).
    warmups_since_clear: Option<f64>,
}

/// Readiness monitors and freeze frame: what an inspection station sees.
#[tauri::command]
async fn read_emissions(state: State<'_, AppState>) -> CmdResult<EmissionsReport> {
    let mut inner = state.0.lock().await;
    let c = inner.client()?;
    let readiness = c.read_readiness().await.ok();
    let freeze_frame = c.read_freeze_frame().await.unwrap_or(None);
    let km_since_clear = c.read_pid(0x31).await.ok();
    let warmups_since_clear = c.read_pid(0x30).await.ok();
    let outcome = match &readiness {
        Some(r) => format!("{} monitors incomplete", r.incomplete()),
        None => "Readiness not available".into(),
    };
    inner.record(
        "Read emissions readiness",
        "Engine (OBD-II)",
        RiskTier::Read,
        true,
        outcome,
    );
    Ok(EmissionsReport {
        readiness,
        freeze_frame,
        km_since_clear,
        warmups_since_clear,
    })
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
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("ApexDiag"))
                .join("snapshots");
            app.manage(SnapshotStore::new(dir));
            Ok(())
        })
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
            scan_module,
            log_quick_test,
            read_module,
            read_emissions,
            clear_module_codes,
            run_function,
            list_snapshots,
            take_snapshot,
            compare_snapshots,
            restore_snapshot,
            activity_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ApexDiag");
}
