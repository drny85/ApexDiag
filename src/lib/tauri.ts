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

export interface ModuleInfo {
  name: string;
  description: string;
  role: string;
  address: number;
  system: SystemId;
}

export interface Catalog {
  tiers: Tier[];
  functions: FunctionInfo[];
  pids: PidInfo[];
  modules: ModuleInfo[];
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

export const api = {
  catalog: () => call<Catalog>("catalog"),
  listDevices: () => call<DiscoveredDevice[]>("list_devices"),
  connect: (id: string) => call<ConnectReport>("connect", { id }),
  disconnect: () => call<void>("disconnect"),
  readLive: (pids: number[]) => call<LiveValue[]>("read_live", { pids }),
  vehicleState: () => call<StateReport>("vehicle_state"),
  setChargerConfirmed: (on: boolean) => call<void>("set_charger_confirmed", { on }),
  simSetEngine: (running: boolean) => call<void>("sim_set_engine", { running }),
  readObdCodes: () => call<ObdCodes>("read_obd_codes"),
  clearObdCodes: () => call<Dtc[]>("clear_obd_codes"),
  scanModules: () => call<ModuleScan[]>("scan_modules"),
  readModule: (address: number) => call<ModuleDetail>("read_module", { address }),
  clearModuleCodes: (address: number) => call<void>("clear_module_codes", { address }),
  runFunction: (id: string) => call<FunctionRun>("run_function", { id }),
  activityLog: () => call<LogEntry[]>("activity_log"),
};
