import { invoke } from "@tauri-apps/api/core";

export type TierId = "read" | "service" | "adapt" | "code" | "security" | "flash";

export type TransportKind =
  | { kind: "ble"; peripheral_id: string }
  | { kind: "serial"; path: string; baud: number }
  | { kind: "doip"; host: string; port: number }
  | { kind: "simulator"; engine_running: boolean }
  | { kind: "mock" };

export interface DiscoveredDevice {
  id: string;
  name: string;
  detail: string;
  transport: TransportKind;
  available: boolean;
}

export interface Requirement {
  kind: string;
  volts?: number;
  text: string;
}

export interface Tier {
  id: TierId;
  label: string;
  explain: string;
  confirmations: number;
}

export interface FunctionInfo {
  id: string;
  name: string;
  tier: TierId;
  tier_label: string;
  what: string;
  why: string;
  risks: string;
  undo: string;
  confirmations: number;
  requirements: Requirement[];
  verified_on_car: boolean;
}

export interface PidInfo {
  pid: number;
  name: string;
  unit: string;
  min: number;
  max: number;
  explain: string;
}

export type SystemId = "powertrain" | "chassis" | "body" | "comfort" | "infotainment" | "safety";

export type ModuleLink = { kind: "bmw_extended" } | { kind: "can11"; tx: number; rx: number };

export interface ModuleInfo {
  name: string;
  description: string;
  role: string;
  address: number;
  system: SystemId;
  link: ModuleLink;
  protocol: "kwp2000" | "uds";
  /** Position on the top-down map (viewBox 240 x 470, front at the top). */
  spot: { x: number; y: number; area: string };
}

export type Make = "bmw" | "mercedes_benz" | "toyota" | "honda" | "jeep" | "generic";

export interface Pack {
  id: string;
  make: Make;
  brand: string;
  model: string;
  chassis: string;
  years: string;
  body: "suv" | "sedan" | "offroad";
  layout: "documented" | "demo";
  dealer_tool: string;
  modules: ModuleInfo[];
  records: RecordDef[];
  jobs: { function_id: string; oem_job: string | null; module: number; status: "verified" | "pending_capture" }[];
  functions: string[];
  notes: string[];
}

export interface VinInfo {
  vin: string;
  well_formed: boolean;
  demo: boolean;
  wmi: string;
  region: string;
  country: string | null;
  make: Make;
  manufacturer: string | null;
  model_year: number | null;
  plant: string | null;
  serial: string;
  check_digit_ok: boolean | null;
  check_digit_required: boolean;
}

export interface VehicleView {
  vin_info: VinInfo | null;
  pack: Pack;
  scan_addresses: number[];
  demo_story: string | null;
}

export type RecordKind = "coding" | "adaptation";

export interface RecordDef {
  module: number;
  id: number;
  name: string;
  kind: RecordKind;
  explain: string;
  status: "verified" | "pending_capture";
}

export interface Catalog {
  tiers: Tier[];
  functions: FunctionInfo[];
  pids: PidInfo[];
  packs: Pack[];
}

export type SnapshotKind = "manual" | "automatic" | "demo";

export interface Snapshot {
  id: string;
  vin: string;
  module_address: number;
  module_name: string;
  taken_at_ms: number;
  kind: SnapshotKind;
  reason: string;
  simulated: boolean;
  identification: string | null;
  records: { id: number; raw: number[] }[];
  faults: { code: string; status: number }[];
  missing: number[];
}

export interface RecordChange {
  id: number;
  name: string;
  kind: RecordKind;
  explain: string;
  before: string | null;
  after: string | null;
  before_value: number | null;
  after_value: number | null;
  delta: number | null;
  changed: boolean;
}

export interface ModuleDiff {
  records: RecordChange[];
  faults: { appeared: string[]; gone: string[]; unchanged: string[] };
  identification_changed: boolean;
  restorable: number;
  changed: number;
}

export interface Comparison {
  from: Snapshot;
  to: Snapshot;
  diff: ModuleDiff;
}

export interface RestoreReport {
  steps: string[];
  written: number;
  undo_id: string | null;
}

export interface AdapterInfo {
  version: string;
  supports_extended_addressing: boolean;
}

export interface MilStatus {
  mil_on: boolean;
  dtc_count: number;
}

export interface ConnectReport {
  device: DiscoveredDevice;
  adapter: AdapterInfo;
  vin: string | null;
  battery_v: number | null;
  mil: MilStatus | null;
  supported_pids: number[];
  simulated: boolean;
  vehicle: VehicleView;
  transcript: string[];
}

export interface LiveValue {
  pid: number;
  value: number;
}

export interface VehicleState {
  speed_kmh: number | null;
  rpm: number | null;
  battery_v: number | null;
  charger_confirmed: boolean;
  wired_adapter: boolean;
  backup_taken: boolean;
  job_verified: boolean;
}

export interface StateReport {
  state: VehicleState;
  engine_running: boolean | null;
  functions: { id: string; unmet: Requirement[] }[];
}

export interface Dtc {
  code: string;
  description: string | null;
}

export interface ObdCodes {
  stored: Dtc[];
  pending: Dtc[];
  permanent: Dtc[];
  mil: MilStatus | null;
}

export interface ModuleScan {
  module: ModuleInfo;
  present: boolean;
  faults: number | null;
  codes: ModuleFault[];
  elapsed_ms: number;
}

export interface Monitor {
  id: string;
  name: string;
  supported: boolean;
  complete: boolean;
  explain: string;
}

export interface Readiness {
  mil_on: boolean;
  dtc_count: number;
  compression_ignition: boolean;
  monitors: Monitor[];
}

export interface EmissionsReport {
  readiness: Readiness | null;
  freeze_frame: { dtc: string; values: [number, number][] } | null;
  km_since_clear: number | null;
  warmups_since_clear: number | null;
}

export interface ModuleFault {
  code: string;
  status: number;
  description: string | null;
}

export interface ModuleDetail {
  module: ModuleInfo;
  identification: string | null;
  faults: ModuleFault[];
}

export interface FunctionRun {
  simulated: boolean;
  steps: string[];
}

export interface LogEntry {
  at_ms: number;
  action: string;
  target: string;
  tier: TierId;
  outcome: string;
  ok: boolean;
}

export type CheckId = "fuel_trims" | "intake_vacuum" | "idle_stability" | "catalyst_oxygen_sensors" | "warm_up" | "charging";
export type Verdict = "pass" | "warn" | "fail" | "unknown";

export interface Fix {
  action: string;
  difficulty: "diy" | "experienced" | "shop";
  cost_usd: [number, number];
  time: string;
  parts: string;
  tools: string;
}

export interface Cause {
  name: string;
  likelihood: "common" | "possible" | "rare";
  why: string;
  fix: Fix;
}

export interface PlanStep {
  id: string;
  title: string;
  detail: string;
  kind: { kind: "auto"; check: CheckId } | { kind: "manual" };
  tools: string | null;
  caution: string | null;
}

export interface Playbook {
  id: string;
  codes: string[];
  title: string;
  meaning: string;
  causes: Cause[];
  steps: PlanStep[];
  verify: string;
  verify_checks: CheckId[];
}

export interface CheckResult {
  check: CheckId;
  verdict: Verdict;
  headline: string;
  detail: string;
  readings: { name: string; value: number; unit: string }[];
}

export interface StepRecord {
  step_id: string;
  title: string;
  outcome: string;
  note: string;
  result: CheckResult | null;
}

export interface RepairRecord {
  id: string;
  vin: string;
  code: string;
  module: string | null;
  title: string;
  started_ms: number;
  finished_ms: number;
  steps: StepRecord[];
  fix: string;
  parts: string;
  verification: CheckResult[];
  verified: boolean;
  simulated: boolean;
}

/** Error shape returned by every Rust command. */
export interface CmdError {
  kind: string;
  message: string;
  unmet: Requirement[];
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function asCmdError(e: unknown): CmdError {
  if (e && typeof e === "object" && "message" in e) {
    const err = e as Partial<CmdError>;
    return { kind: err.kind ?? "error", message: String(err.message), unmet: err.unmet ?? [] };
  }
  return { kind: "error", message: String(e), unmet: [] };
}

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) {
    return Promise.reject({ kind: "browser", message: "Open ApexDiag as a desktop app to talk to a car.", unmet: [] });
  }
  return invoke<T>(cmd, args);
}

export interface CarTestStepInfo {
  id: string;
  title: string;
  what: string;
  why: string;
}

export interface CarTestResult {
  id: string;
  ok: boolean;
  skipped: boolean;
  headline: string;
  details: string[];
  data: unknown;
  elapsed_ms: number;
}

export interface CaptureView {
  path: string | null;
  bytes: number;
  tail: string[];
}

export interface SavedTest {
  folder: string;
  report: string;
  capture: string | null;
}

export interface MarkInfo {
  mark_ms: number;
  before_ms: number;
  after_ms: number;
}

export interface ReplayFrame {
  t_ms: number;
  values: [number, number][];
}

export interface ReplayChange {
  pid: number;
  name: string;
  unit: string;
  usual: number;
  peak: number;
  offset_ms: number;
  score: number;
  text: string;
}

export interface ReplayCode {
  code: string;
  status: number | null;
  description: string | null;
}

export interface ReplayModule {
  name: string;
  address: number;
  present: boolean;
  codes: ReplayCode[];
}

export interface Replay {
  id: string;
  vin: string;
  vehicle: string;
  simulated: boolean;
  mark_ms: number;
  note: string;
  frames: ReplayFrame[];
  changes: ReplayChange[];
  engine_stored: ReplayCode[];
  engine_pending: ReplayCode[];
  modules: ReplayModule[];
}

export interface ReplaySummary {
  id: string;
  vin: string;
  vehicle: string;
  simulated: boolean;
  mark_ms: number;
  note: string;
  seconds: number;
  headline: string | null;
  changes: number;
  faults: number;
}

export type Grade = "not_checked" | "good" | "attention" | "problem";

export interface InspectionCode {
  code: string;
  description: string | null;
}

export interface InspectionSection {
  id: "engine" | "modules" | "readiness" | "cleared" | "battery" | "mileage";
  title: string;
  grade: Grade;
  headline: string;
  details: string[];
  why: string;
}

export interface InspectionView {
  report: { overall: Grade; summary: string; sections: InspectionSection[] };
  readings: {
    km_since_clear: number | null;
    warmups_since_clear: number | null;
    battery_v: number | null;
    engine_running: boolean | null;
    odometer_km: number | null;
    modules: { name: string; system: string; present: boolean; faults: InspectionCode[] }[];
  };
  vin: string | null;
  vehicle: string;
  simulated: boolean;
  at_ms: number;
}

export type ScanStatus = "not_read" | "info" | "good" | "attention" | "problem";
export type ScanArea = "engine" | "emissions" | "electrical" | "modules" | "vehicle";

export interface ScanItem {
  id: string;
  area: ScanArea;
  title: string;
  value: string | null;
  status: ScanStatus;
  headline: string;
  what: string;
  problem: string | null;
  fix: string[];
  facts: { label: string; value: string }[];
  guide: { code: string; playbook: boolean; title: string } | null;
  module: string | null;
}

export interface FullScanView {
  report: {
    overall: ScanStatus;
    title: string;
    summary: string;
    counts: { problem: number; attention: number; good: number; info: number; not_read: number };
    items: ScanItem[];
    tips: string[];
  };
  vin: string | null;
  vehicle: string;
  simulated: boolean;
  at_ms: number;
}

export interface FullScanStarted {
  engine_running: boolean | null;
  values: number;
  codes: number;
}

export const api = {
  catalog: () => call<Catalog>("catalog"),
  listDevices: () => call<DiscoveredDevice[]>("list_devices"),
  scanBle: (all = false) => call<DiscoveredDevice[]>("scan_ble", { all }),
  connect: (id: string) => call<ConnectReport>("connect", { id }),
  disconnect: () => call<void>("disconnect"),
  readLive: (pids: number[]) => call<LiveValue[]>("read_live", { pids }),
  vehicleState: () => call<StateReport>("vehicle_state"),
  setChargerConfirmed: (on: boolean) => call<void>("set_charger_confirmed", { on }),
  simSetEngine: (running: boolean) => call<void>("sim_set_engine", { running }),
  readObdCodes: () => call<ObdCodes>("read_obd_codes"),
  clearObdCodes: () => call<Dtc[]>("clear_obd_codes"),
  scanModules: () => call<ModuleScan[]>("scan_modules"),
  scanModule: (address: number) => call<ModuleScan>("scan_module", { address }),
  logQuickTest: (found: number, faults: number) => call<void>("log_quick_test", { found, faults }),
  readEmissions: () => call<EmissionsReport>("read_emissions"),
  readModule: (address: number) => call<ModuleDetail>("read_module", { address }),
  clearModuleCodes: (address: number) => call<void>("clear_module_codes", { address }),
  runFunction: (id: string) => call<FunctionRun>("run_function", { id }),
  listSnapshots: () => call<Snapshot[]>("list_snapshots"),
  takeSnapshot: (address: number) => call<Snapshot>("take_snapshot", { address }),
  compareSnapshots: (address: number, from: string, to: string) => call<Comparison>("compare_snapshots", { address, from, to }),
  restoreSnapshot: (address: number, id: string) => call<RestoreReport>("restore_snapshot", { address, id }),
  activityLog: () => call<LogEntry[]>("activity_log"),
  copilotPlan: (code: string) => call<Playbook>("copilot_plan", { code }),
  copilotRunCheck: (check: CheckId) => call<CheckResult>("copilot_run_check", { check }),
  simApplyRepair: (code: string) => call<boolean>("sim_apply_repair", { code }),
  saveRepair: (record: RepairRecord) => call<RepairRecord>("save_repair", { record }),
  listRepairs: () => call<RepairRecord[]>("list_repairs"),
  carTestPlan: () => call<CarTestStepInfo[]>("car_test_plan"),
  carTestStep: (step: string) => call<CarTestResult>("car_test_step", { step }),
  captureInfo: () => call<CaptureView>("capture_info"),
  saveCarTest: (report: unknown) => call<SavedTest>("save_car_test", { report }),
  replayMark: (note?: string) => call<MarkInfo>("replay_mark", { note: note ?? null }),
  replayCancel: () => call<void>("replay_cancel"),
  replayFinish: (modules: boolean) => call<Replay>("replay_finish", { modules }),
  listReplays: () => call<ReplaySummary[]>("list_replays"),
  getReplay: (id: string) => call<Replay>("get_replay", { id }),
  deleteReplay: (id: string) => call<void>("delete_replay", { id }),
  simStumble: () => call<void>("sim_stumble"),
  simDrive: (on: boolean) => call<void>("sim_drive", { on }),
  inspectionStart: () => call<void>("inspection_start"),
  inspectionFinish: (modules: { address: number; present: boolean; codes: InspectionCode[] }[], lang: "en" | "es" = "en") =>
    call<InspectionView>("inspection_finish", { modules, lang }),
  fullScanStart: () => call<FullScanStarted>("full_scan_start"),
  fullScanFinish: (modules: { address: number; present: boolean; codes: ModuleFault[] }[], lang: "en" | "es" = "en") =>
    call<FullScanView>("full_scan_finish", { modules, lang }),
  saveDocument: (name: string, contents: string) => call<string>("save_document", { name, contents }),
};
