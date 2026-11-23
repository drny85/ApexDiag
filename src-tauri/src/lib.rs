//! Tauri shell: exposes the Rust diagnostic core to the React UI.
//!
//! Every command that changes something in the car goes through
//! [`Inner::gate`], which reads the car's live state and refuses to run when
//! any safety requirement is not proven. Every attempt, allowed or blocked,
//! is written to the activity log.

mod car_test;
mod repairs;
mod replays;
mod timemachine;

use protocol_core::catalog::{self, Function, Requirement, RiskTier, VehicleState};
use protocol_core::copilot::{self, Check, CheckResult, Playbook, RepairRecord, Samples};
use protocol_core::full_scan;
use protocol_core::inspection::{self, Readings};
use protocol_core::obd2::{self, Dtc, MilStatus, PidInfo, Readiness, Service};
use protocol_core::replay;
use protocol_core::snapshot::{
    self, ModuleDiff, RecordKind, RecordValue, Snapshot, SnapshotFault, SnapshotKind,
};
use protocol_core::vehicle::{self, JobStatus, Layout, Module, Pack, System};
use protocol_core::vin::{self, VinInfo};
use repairs::RepairStore;
use replays::{Replay, ReplayCode, ReplayModule, ReplayStore, ReplaySummary};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};
use timemachine::SnapshotStore;
use tokio::sync::Mutex;
use transport::capture::Capture;
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
    /// The raw capture file for this connection.
    capture: Option<std::path::PathBuf>,
    /// The last minute of live data, for Instant Replay.
    replay: replay::Buffer,
    /// A moment the driver marked that has not been saved yet.
    mark: Option<(u64, String)>,
    /// Inspection report: what was read before the module scan.
    inspection: Option<Readings>,
    /// Service 01 PIDs the engine module said it supports, read once at
    /// connect so later scans only ask for values the car has.
    supported_pids: Vec<u8>,
    /// Full scan: what was read before the module scan.
    full_scan: Option<full_scan::Input>,
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

/// What went wrong talking to the adapter, in words a technician can act on.
fn explain_client_error(e: &ClientError) -> String {
    use protocol_core::ProtocolError;
    use transport::TransportError;
    match e {
        ClientError::Protocol(ProtocolError::NegativeResponse { nrc, .. }) => {
            protocol_core::diag::Nrc(*nrc).explain().to_string()
        }
        ClientError::Transport(TransportError::Timeout(_)) => {
            "The adapter stopped answering. Check it is still plugged in, its light is on and the ignition is on, then try again.".into()
        }
        ClientError::Transport(TransportError::NotConnected) => {
            "The adapter is not connected. Connect it again from the Connect page.".into()
        }
        ClientError::Transport(other) => format!("The link to the adapter failed: {other}"),
        ClientError::NoReply(name) => format!(
            "{name} did not answer. It may not be fitted to this car, or the ignition is off."
        ),
        ClientError::Unsupported(cmd) => format!(
            "This adapter does not support {cmd}. Many low-cost ELM327 copies leave commands out; engine OBD-II still works."
        ),
        other => other.to_string(),
    }
}

impl From<ClientError> for CmdError {
    fn from(e: ClientError) -> Self {
        Self::new("device", explain_client_error(&e))
    }
}

/// How a module is reached, for reports.
fn link_label(m: &Module) -> String {
    match m.link {
        vehicle::Link::BmwExtended => format!(
            "BMW D-CAN extended addressing (6F1 to address {:02X})",
            m.address
        ),
        vehicle::Link::Can11 { tx, rx } => format!("CAN {tx:03X} / {rx:03X}"),
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
    // Bluetooth LE adapters from the last scan, ahead of the practice cars.
    let seen = ble_seen().lock().map(|v| v.clone()).unwrap_or_default();
    if seen.is_empty() {
        out.insert(
            0,
            DiscoveredDevice {
                id: "ble-scan".into(),
                name: "Bluetooth OBD adapter".into(),
                detail: "Veepeak OBDCheck BLE, vLinker, OBDLink CX and other ELM327 BLE adapters. \
                         Plug it in, turn the ignition on, then scan."
                    .into(),
                transport: TransportKind::Ble {
                    peripheral_id: String::new(),
                },
                available: false,
            },
        );
    } else {
        out.splice(0..0, seen);
    }
    out
}

/// Adapters found by the last Bluetooth scan.
fn ble_seen() -> &'static std::sync::Mutex<Vec<DiscoveredDevice>> {
    static SEEN: std::sync::OnceLock<std::sync::Mutex<Vec<DiscoveredDevice>>> =
        std::sync::OnceLock::new();
    SEEN.get_or_init(Default::default)
}

/// Scans for Bluetooth LE adapters for a few seconds. With `all`, lists every
/// named Bluetooth device, not only ones that look like OBD adapters.
#[tauri::command]
async fn scan_ble(all: Option<bool>) -> CmdResult<Vec<DiscoveredDevice>> {
    let found = transport::ble::scan(std::time::Duration::from_secs(4), all.unwrap_or(false))
        .await
        .map_err(|e| CmdError::new("bluetooth", e.to_string()))?;
    if let Ok(mut seen) = ble_seen().lock() {
        *seen = found.clone();
    }
    Ok(found)
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
    captures: State<'_, Captures>,
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
        (TransportKind::Ble { peripheral_id }, None) if !peripheral_id.is_empty() => {
            Box::new(transport::ble::BleTransport::new(peripheral_id.clone()))
        }
        (TransportKind::Ble { .. }, None) => {
            return Err(CmdError::new(
                "unavailable",
                "Scan for Bluetooth adapters first.",
            ))
        }
        _ => {
            return Err(CmdError::new(
                "unavailable",
                "This adapter is not supported yet.",
            ))
        }
    };
    // Record every exchange, so real-car data can be checked afterwards.
    let (link, capture_path) = match captures.open(&device.name) {
        Some((file, path)) => (
            Box::new(Capture::new(link, file, &device.name, now_ms()))
                as Box<dyn HardwareTransport + Send>,
            Some(path),
        ),
        None => (link, None),
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
    inner.capture = capture_path;
    inner.replay.clear();
    inner.mark = None;
    inner.supported_pids = supported_pids.clone();
    inner.full_scan = None;
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
    inner.replay.clear();
    inner.mark = None;
    inner.supported_pids.clear();
    inner.full_scan = None;
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
    // Up to six PIDs per request where the car allows it.
    let out: Vec<LiveValue> = c
        .read_pids(&pids)
        .await
        .into_iter()
        .map(|(pid, value)| LiveValue { pid, value })
        .collect();
    let frame = out.iter().map(|v| (v.pid, v.value)).collect();
    inner.replay.push(now_ms() as u64, frame);
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
    // Ask for the fault memory straight away: any reply, even a refusal,
    // proves the module is there, so fitted modules cost one round trip
    // instead of two. Only when that gets no clean answer is Tester Present
    // asked, so "present" means exactly what it did before.
    let (present, dtcs) = match c.read_module_dtcs(m).await {
        Ok(d) => (true, Some(d)),
        Err(ClientError::Protocol(protocol_core::ProtocolError::NegativeResponse { .. })) => {
            (true, None)
        }
        // The adapter can't address this module at all (no AT CEA on a
        // BMW), or the link failed: nothing to gain from asking again.
        Err(ClientError::Unsupported(_)) | Err(ClientError::Transport(_)) => (false, None),
        Err(_) => (c.probe(m).await.unwrap_or(false), None),
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

// ---- commands: Repair Copilot ------------------------------------------

/// The guided plan for a fault code.
#[tauri::command]
fn copilot_plan(code: String) -> Playbook {
    copilot::plan_for(code.trim())
}

/// Samples the check's live values for a few seconds and judges them.
#[tauri::command]
async fn copilot_run_check(state: State<'_, AppState>, check: Check) -> CmdResult<CheckResult> {
    let mut inner = state.0.lock().await;
    let c = inner.client()?;
    let mut samples = Samples::new();
    let until = std::time::Instant::now() + std::time::Duration::from_millis(check.sample_ms());
    loop {
        for &pid in check.pids() {
            if let Ok(v) = c.read_pid(pid).await {
                samples.entry(pid).or_default().push(v);
            }
        }
        if std::time::Instant::now() >= until {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    }
    let r = copilot::evaluate(check, &samples);
    let outcome = format!("{:?}: {}", r.verdict, r.headline);
    inner.record(
        "Repair Copilot check",
        check.title(),
        RiskTier::Read,
        true,
        outcome,
    );
    Ok(r)
}

/// Demo only: pretend the repair was done on the practice car.
#[tauri::command]
async fn sim_apply_repair(state: State<'_, AppState>, code: String) -> CmdResult<bool> {
    let mut inner = state.0.lock().await;
    let sim = inner.client()?.link_mut().simulator().ok_or_else(|| {
        CmdError::new(
            "unavailable",
            "Only a practice car can be repaired from the app.",
        )
    })?;
    let done = sim.apply_demo_repair(&code);
    inner.record(
        "Demo: apply repair",
        &code,
        RiskTier::Read,
        true,
        if done {
            "Practice car repaired"
        } else {
            "Nothing to repair for this code"
        },
    );
    Ok(done)
}

#[tauri::command]
async fn save_repair(
    state: State<'_, AppState>,
    repairs: State<'_, RepairStore>,
    mut record: RepairRecord,
) -> CmdResult<RepairRecord> {
    let mut inner = state.0.lock().await;
    record.vin = inner.vin()?;
    record.simulated = inner.simulated();
    if record.id.is_empty() {
        record.id = format!("r{}", now_ms());
    }
    repairs
        .save(&record)
        .map_err(|e| CmdError::new("storage", format!("Could not save the repair: {e}")))?;
    let outcome = if record.verified {
        "Saved, fix verified"
    } else {
        "Saved, not verified yet"
    };
    inner.record(
        "Save repair record",
        &record.code,
        RiskTier::Read,
        true,
        outcome,
    );
    Ok(record)
}

#[tauri::command]
async fn list_repairs(
    state: State<'_, AppState>,
    repairs: State<'_, RepairStore>,
) -> CmdResult<Vec<RepairRecord>> {
    let inner = state.0.lock().await;
    Ok(repairs.list(&inner.vin()?))
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

// ---- raw capture and the first real car test --------------------------

/// Where raw captures are kept: the newest few, one file per connection.
struct Captures {
    dir: std::path::PathBuf,
}

impl Captures {
    const KEEP: usize = 30;

    fn open(&self, adapter: &str) -> Option<(std::fs::File, std::path::PathBuf)> {
        std::fs::create_dir_all(&self.dir).ok()?;
        self.prune();
        let slug: String = adapter
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let path = self.dir.join(format!("capture-{}-{slug}.log", now_ms()));
        let file = std::fs::File::create(&path).ok()?;
        Some((file, path))
    }

    fn prune(&self) {
        let Ok(rd) = std::fs::read_dir(&self.dir) else {
            return;
        };
        let mut files: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "log"))
            .collect();
        files.sort();
        let excess = files.len().saturating_sub(Self::KEEP - 1);
        for p in files.into_iter().take(excess) {
            let _ = std::fs::remove_file(p);
        }
    }
}

#[tauri::command]
fn car_test_plan() -> &'static [car_test::StepInfo] {
    car_test::STEPS
}

#[tauri::command]
async fn car_test_step(
    state: State<'_, AppState>,
    step: String,
) -> CmdResult<car_test::StepResult> {
    let mut inner = state.0.lock().await;
    let r = car_test::run(&mut inner, &step).await?;
    if step == "modules" {
        let target = inner.vin.clone().unwrap_or_else(|| "vehicle".into());
        inner.record(
            "Car test",
            &target,
            RiskTier::Read,
            true,
            "First real car test finished (read only)",
        );
    }
    Ok(r)
}

#[derive(Serialize)]
struct CaptureView {
    path: Option<String>,
    bytes: u64,
    /// The end of the capture, for a quick look in the app.
    tail: Vec<String>,
}

#[tauri::command]
async fn capture_info(state: State<'_, AppState>) -> CmdResult<CaptureView> {
    let inner = state.0.lock().await;
    let Some(path) = inner.capture.clone() else {
        return Ok(CaptureView {
            path: None,
            bytes: 0,
            tail: vec![],
        });
    };
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let tail = lines[lines.len().saturating_sub(40)..]
        .iter()
        .map(|l| l.to_string())
        .collect();
    Ok(CaptureView {
        path: Some(path.display().to_string()),
        bytes: text.len() as u64,
        tail,
    })
}

#[derive(Serialize)]
struct SavedTest {
    folder: String,
    report: String,
    capture: Option<String>,
}

/// Saves the test report and a copy of the raw capture where the person can
/// reach them: Documents/ApexDiag on a Mac, the app's folder in Files on an
/// iPad.
/// Where reports are saved: Documents where the platform has one (the
/// app's own folder on iPad, visible in Files), else ~/Documents, else the
/// app's data folder.
fn documents_folder(app: &tauri::AppHandle) -> CmdResult<std::path::PathBuf> {
    let base = app
        .path()
        .document_dir()
        .or_else(|_| app.path().home_dir().map(|h| h.join("Documents")))
        .or_else(|_| app.path().app_data_dir())
        .map_err(|e| CmdError::new("storage", format!("No folder to save to: {e}")))?;
    let folder = if cfg!(mobile) {
        base
    } else {
        base.join("ApexDiag")
    };
    std::fs::create_dir_all(&folder).map_err(|e| {
        CmdError::new(
            "storage",
            format!("Could not create {}: {e}", folder.display()),
        )
    })?;
    Ok(folder)
}

#[tauri::command]
async fn save_car_test(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    report: serde_json::Value,
) -> CmdResult<SavedTest> {
    let inner = state.0.lock().await;
    let folder = documents_folder(&app)?;
    let stamp = now_ms();
    let name = inner.vin.clone().unwrap_or_else(|| "car".into());
    let report_path = folder.join(format!("car-test-{name}-{stamp}.json"));
    let body = serde_json::to_string_pretty(&report).unwrap_or_default();
    std::fs::write(&report_path, body)
        .map_err(|e| CmdError::new("storage", format!("Could not save the report: {e}")))?;
    let capture = match &inner.capture {
        Some(src) => {
            let dst = folder.join(format!("car-test-{name}-{stamp}.log"));
            std::fs::copy(src, &dst).map_err(|e| {
                CmdError::new("storage", format!("Could not copy the capture: {e}"))
            })?;
            Some(dst.display().to_string())
        }
        None => None,
    };
    Ok(SavedTest {
        folder: folder.display().to_string(),
        report: report_path.display().to_string(),
        capture,
    })
}

// ---- commands: Instant Replay -------------------------------------------

#[derive(Serialize)]
struct MarkInfo {
    mark_ms: u64,
    /// How much history before the mark is already in memory.
    before_ms: u64,
    /// How long recording continues after the mark.
    after_ms: u64,
}

/// Marks this moment. The 30 seconds before it are already in memory;
/// recording continues for 30 seconds after, then `replay_finish` saves it.
#[tauri::command]
async fn replay_mark(state: State<'_, AppState>, note: Option<String>) -> CmdResult<MarkInfo> {
    let mut inner = state.0.lock().await;
    let c = inner.client()?;
    c.link_mut().capture_note("Instant Replay: moment marked");
    let mark_ms = now_ms() as u64;
    let oldest = inner
        .replay
        .window(mark_ms.saturating_sub(replay::BEFORE_MS), mark_ms)
        .first()
        .map(|f| f.t_ms)
        .unwrap_or(mark_ms);
    inner.mark = Some((mark_ms, note.unwrap_or_default().trim().to_string()));
    inner.record(
        "Instant Replay: mark",
        "Live data",
        RiskTier::Read,
        true,
        "Moment marked",
    );
    Ok(MarkInfo {
        mark_ms,
        before_ms: mark_ms - oldest,
        after_ms: replay::AFTER_MS,
    })
}

#[tauri::command]
async fn replay_cancel(state: State<'_, AppState>) -> CmdResult<()> {
    state.0.lock().await.mark = None;
    Ok(())
}

fn engine_codes(list: Vec<Dtc>) -> Vec<ReplayCode> {
    list.into_iter()
        .map(|d| ReplayCode {
            code: d.code,
            status: None,
            description: d.description.map(Into::into),
        })
        .collect()
}

/// Saves the marked moment: the live data around it, what changed, and the
/// fault memory read now. Read-only: nothing is cleared or written.
#[tauri::command]
async fn replay_finish(
    state: State<'_, AppState>,
    store: State<'_, ReplayStore>,
    modules: bool,
) -> CmdResult<Replay> {
    let mut inner = state.0.lock().await;
    let (mark_ms, note) = inner
        .mark
        .take()
        .ok_or_else(|| CmdError::new("no_mark", "Mark a moment first."))?;
    let frames = inner.replay.replay_window(mark_ms);
    let changes = replay::what_changed(&frames, mark_ms);
    let list = if modules {
        scan_list(&inner)
    } else {
        Vec::new()
    };
    let simulated = inner.simulated();
    let pack = inner.pack();
    let vin = inner.vin.clone().unwrap_or_default();
    let c = inner.client()?;
    c.link_mut()
        .capture_note("Instant Replay: reading fault memory");
    let stored = c
        .read_obd_dtcs(Service::StoredDtcs)
        .await
        .unwrap_or_default();
    let pending = c
        .read_obd_dtcs(Service::PendingDtcs)
        .await
        .unwrap_or_default();
    let mut scanned = Vec::new();
    for m in list {
        let s = scan_one(c, simulated, pack, m).await;
        scanned.push(ReplayModule {
            name: m.name.to_string(),
            address: m.address,
            present: s.present,
            codes: s
                .codes
                .into_iter()
                .map(|f| ReplayCode {
                    code: f.code,
                    status: Some(f.status),
                    description: f.description.map(Into::into),
                })
                .collect(),
        });
    }
    let replay = Replay {
        id: format!("replay-{mark_ms}"),
        vin,
        vehicle: format!("{} {}", pack.brand, pack.model),
        simulated,
        mark_ms,
        note,
        frames,
        changes,
        engine_stored: engine_codes(stored),
        engine_pending: engine_codes(pending),
        modules: scanned,
    };
    store
        .save(&replay)
        .map_err(|e| CmdError::new("storage", format!("Could not save the replay: {e}")))?;
    let summary = replay.summary();
    inner.record(
        "Instant Replay: save",
        "Live data",
        RiskTier::Read,
        true,
        format!(
            "{:.0} s saved, {} changes, {} faults",
            summary.seconds, summary.changes, summary.faults
        ),
    );
    Ok(replay)
}

#[tauri::command]
fn list_replays(store: State<'_, ReplayStore>) -> Vec<ReplaySummary> {
    store.list()
}

#[tauri::command]
fn get_replay(store: State<'_, ReplayStore>, id: String) -> CmdResult<Replay> {
    store
        .get(&id)
        .ok_or_else(|| CmdError::new("unknown", "That replay is no longer saved."))
}

#[tauri::command]
fn delete_replay(store: State<'_, ReplayStore>, id: String) -> CmdResult<()> {
    store
        .delete(&id)
        .map_err(|e| CmdError::new("storage", format!("Could not delete the replay: {e}")))
}

/// Demo only: makes the practice car's engine stumble once.
#[tauri::command]
async fn sim_stumble(state: State<'_, AppState>) -> CmdResult<()> {
    let mut inner = state.0.lock().await;
    let sim =
        inner.client()?.link_mut().simulator().ok_or_else(|| {
            CmdError::new("unavailable", "Only the demo car can be made to stumble.")
        })?;
    sim.stumble();
    inner.record(
        "Demo: engine stumble",
        "simulator",
        RiskTier::Read,
        true,
        "Done",
    );
    Ok(())
}

// ---- commands: Inspection report ----------------------------------------

/// Step one of the inspection: everything except the module scan, which the
/// page runs module by module (with `scan_module`) to show progress.
/// Read-only.
#[tauri::command]
async fn inspection_start(state: State<'_, AppState>) -> CmdResult<()> {
    let mut inner = state.0.lock().await;
    let c = inner.client()?;
    c.link_mut()
        .capture_note("Inspection report: engine, readiness, battery");
    let stored = c.read_obd_dtcs(Service::StoredDtcs).await.ok();
    let pending = c
        .read_obd_dtcs(Service::PendingDtcs)
        .await
        .unwrap_or_default();
    let permanent = c
        .read_obd_dtcs(Service::PermanentDtcs)
        .await
        .unwrap_or_default();
    let readiness = c.read_readiness().await.ok();
    let km_since_clear = c.read_pid(0x31).await.ok();
    let warmups_since_clear = c.read_pid(0x30).await.ok();
    let km_with_mil = c.read_pid(0x21).await.ok();
    let odometer_km = c.read_pid(0xA6).await.ok();
    let rpm = c.read_pid(0x0C).await.ok();
    let battery_v = match c.read_pid(0x42).await {
        Ok(v) => Some(v),
        Err(_) => c.battery_voltage().await.ok().map(|v| v as f64),
    };
    let codes = |list: Vec<Dtc>| {
        list.into_iter()
            .map(|d| inspection::Code {
                code: d.code,
                description: d.description.map(Into::into),
            })
            .collect::<Vec<_>>()
    };
    inner.inspection = Some(Readings {
        mil_on: readiness.as_ref().map(|r| r.mil_on),
        stored: stored.map(codes),
        pending: codes(pending),
        permanent: codes(permanent),
        readiness,
        km_since_clear,
        warmups_since_clear,
        km_with_mil,
        battery_v,
        engine_running: rpm.map(|r| r > 300.0),
        odometer_km,
        modules: Vec::new(),
        modules_not_scanned: 0,
    });
    Ok(())
}

#[derive(serde::Deserialize)]
struct ScannedModule {
    address: u8,
    present: bool,
    codes: Vec<inspection::Code>,
}

#[derive(Serialize)]
struct InspectionView {
    report: inspection::Report,
    readings: Readings,
    vin: Option<String>,
    vehicle: String,
    simulated: bool,
    at_ms: u64,
}

/// Step two: grades everything read so far plus the page's module scan.
#[tauri::command]
async fn inspection_finish(
    state: State<'_, AppState>,
    modules: Vec<ScannedModule>,
    lang: Option<inspection::Lang>,
) -> CmdResult<InspectionView> {
    let mut inner = state.0.lock().await;
    let mut readings = inner
        .inspection
        .take()
        .ok_or_else(|| CmdError::new("not_started", "Start the inspection first."))?;
    let pack = inner.pack();
    let allowed = scan_list(&inner);
    readings.modules = modules
        .into_iter()
        .filter_map(|m| {
            let info = pack.module(m.address)?;
            Some(inspection::ModuleResult {
                name: info.name.to_string(),
                system: info.system,
                present: m.present,
                faults: m.codes,
            })
        })
        .collect();
    readings.modules_not_scanned = pack.modules.len().saturating_sub(allowed.len());
    let report = inspection::grade(&readings, lang.unwrap_or_default());
    inner.record(
        "Inspection report",
        "Whole car",
        RiskTier::Read,
        true,
        report.summary.clone(),
    );
    Ok(InspectionView {
        report,
        readings,
        vin: inner.vin.clone(),
        vehicle: format!("{} {}", pack.brand, pack.model),
        simulated: inner.simulated(),
        at_ms: now_ms() as u64,
    })
}

// ---- commands: Full scan --------------------------------------------------

#[derive(Serialize)]
struct FullScanStarted {
    engine_running: Option<bool>,
    /// Live values read, for the progress line.
    values: usize,
    codes: usize,
}

/// Step one of the full scan: everything the engine module can tell over
/// OBD-II, read with as few requests as possible (see
/// [`transport::full_scan::read_engine`]). The page then scans the modules
/// one by one for progress. Read-only.
#[tauri::command]
async fn full_scan_start(state: State<'_, AppState>) -> CmdResult<FullScanStarted> {
    let mut inner = state.0.lock().await;
    let supported = inner.supported_pids.clone();
    let c = inner.client()?;
    c.link_mut()
        .capture_note("Full scan: readiness, codes, live values, battery");
    let e = transport::full_scan::read_engine(c, &supported).await;
    let adapter = c.info().cloned();
    inner.replay.push(now_ms() as u64, e.live.to_vec());
    let pack = inner.pack();
    let started = FullScanStarted {
        engine_running: e.engine_running,
        values: e.live.len(),
        codes: e.stored.as_ref().map_or(0, Vec::len) + e.pending.len(),
    };
    inner.full_scan = Some(full_scan::Input {
        vin: inner.vin.as_deref().map(|v| vin::decode(v, current_year())),
        vehicle: format!("{} {}", pack.brand, pack.model),
        is_bmw: pack
            .modules
            .iter()
            .any(|m| m.link == vehicle::Link::BmwExtended),
        simulated: inner.simulated(),
        adapter_version: adapter.as_ref().map(|a| a.version.clone()),
        adapter_extended: adapter.map(|a| a.supports_extended_addressing),
        readiness: e.readiness,
        stored: e.stored,
        pending: e.pending,
        permanent: e.permanent,
        freeze_frame: e.freeze_frame,
        battery_v: e.battery_v,
        engine_running: e.engine_running,
        live: e.live,
        modules: Vec::new(),
        modules_not_scanned: 0,
    });
    Ok(started)
}

#[derive(serde::Deserialize)]
struct FullScanCode {
    code: String,
    status: u8,
    description: Option<String>,
}

#[derive(serde::Deserialize)]
struct FullScanModule {
    address: u8,
    present: bool,
    codes: Vec<FullScanCode>,
}

#[derive(Serialize)]
struct FullScanView {
    report: full_scan::Report,
    vin: Option<String>,
    vehicle: String,
    simulated: bool,
    at_ms: u64,
}

/// Step two: adds the page's module scan and builds the results page.
#[tauri::command]
async fn full_scan_finish(
    state: State<'_, AppState>,
    modules: Vec<FullScanModule>,
    lang: Option<inspection::Lang>,
) -> CmdResult<FullScanView> {
    let mut inner = state.0.lock().await;
    let mut input = inner
        .full_scan
        .take()
        .ok_or_else(|| CmdError::new("not_started", "Start the full scan first."))?;
    let pack = inner.pack();
    let allowed = scan_list(&inner);
    input.modules = modules
        .into_iter()
        .filter_map(|m| {
            let info = pack.module(m.address)?;
            Some(full_scan::ModuleRead {
                name: info.name.to_string(),
                description: info.description.to_string(),
                role: info.role.to_string(),
                system: info.system,
                present: m.present,
                codes: m
                    .codes
                    .into_iter()
                    .map(|c| full_scan::ModuleCode {
                        code: c.code,
                        description: c.description,
                        state: match info.protocol {
                            protocol_core::diag::Protocol::Uds => {
                                full_scan::CodeState::from_uds(c.status)
                            }
                            protocol_core::diag::Protocol::Kwp2000 => full_scan::CodeState::Stored,
                        },
                    })
                    .collect(),
            })
        })
        .collect();
    input.modules_not_scanned = pack.modules.len().saturating_sub(allowed.len());
    let report = full_scan::build(&input, lang.unwrap_or_default());
    let found = input.modules.iter().filter(|m| m.present).count();
    let faults: usize = input.modules.iter().map(|m| m.codes.len()).sum();
    inner.record(
        "Full scan",
        "Whole car",
        RiskTier::Read,
        true,
        format!(
            "{found} modules answered, {faults} module faults; {} problems, {} to check",
            report.counts.problem, report.counts.attention
        ),
    );
    Ok(FullScanView {
        report,
        vin: inner.vin.clone(),
        vehicle: format!("{} {}", pack.brand, pack.model),
        simulated: inner.simulated(),
        at_ms: now_ms() as u64,
    })
}

/// Saves a report the page has rendered (HTML for people, JSON for tools)
/// to the documents folder. Returns the saved path.
#[tauri::command]
async fn save_document(app: tauri::AppHandle, name: String, contents: String) -> CmdResult<String> {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let ext_ok = clean.ends_with(".html") || clean.ends_with(".json");
    if clean.starts_with('.') || !ext_ok {
        return Err(CmdError::new(
            "storage",
            "Only .html and .json reports can be saved.",
        ));
    }
    let path = documents_folder(&app)?.join(clean);
    std::fs::write(&path, contents)
        .map_err(|e| CmdError::new("storage", format!("Could not save the report: {e}")))?;
    Ok(path.display().to_string())
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
            app.manage(RepairStore::new(dir.with_file_name("repairs")));
            app.manage(ReplayStore::new(dir.with_file_name("replays")));
            app.manage(Captures {
                dir: dir.with_file_name("captures"),
            });
            app.manage(SnapshotStore::new(dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            catalog,
            list_devices,
            scan_ble,
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
            copilot_plan,
            copilot_run_check,
            sim_apply_repair,
            save_repair,
            list_repairs,
            car_test_plan,
            car_test_step,
            capture_info,
            save_car_test,
            replay_mark,
            replay_cancel,
            replay_finish,
            list_replays,
            get_replay,
            delete_replay,
            sim_stumble,
            inspection_start,
            inspection_finish,
            full_scan_start,
            full_scan_finish,
            save_document,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ApexDiag");
}
