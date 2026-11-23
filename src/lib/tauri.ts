import { invoke } from "@tauri-apps/api/core";

export type TransportKind =
  | { kind: "ble"; peripheral_id: string }
  | { kind: "serial"; path: string; baud: number }
  | { kind: "doip"; host: string; port: number }
  | { kind: "mock" };

export interface DiscoveredDevice {
  name: string;
  transport: TransportKind;
}

export interface ModuleInfo {
  name: string;
  description: string;
  address: number;
  response_id: number;
}

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Calls a Rust command, or returns `fallback` when running in a plain browser. */
async function call<T>(cmd: string, fallback: T): Promise<T> {
  return inTauri ? invoke<T>(cmd) : fallback;
}

export const api = {
  listDevices: () =>
    call<DiscoveredDevice[]>("list_devices", [{ name: "Simulator", transport: { kind: "mock" } }]),
  bmwE70Modules: () => call<ModuleInfo[]>("bmw_e70_modules", []),
};
