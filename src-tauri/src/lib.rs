//! Tauri shell: exposes the Rust diagnostic core to the React UI as commands.

use protocol_core::bmw_e70;
use serde::Serialize;
use transport::{DiscoveredDevice, TransportKind};

#[derive(Serialize)]
struct ModuleInfo {
    name: &'static str,
    description: &'static str,
    address: u8,
    response_id: u32,
}

/// Lists adapters the user can connect to. Real BLE and serial discovery
/// arrives in step 3; for now only the built-in simulator is offered.
#[tauri::command]
fn list_devices() -> Vec<DiscoveredDevice> {
    vec![DiscoveredDevice {
        name: "Simulator".into(),
        transport: TransportKind::Mock,
    }]
}

/// The E70 modules the BMW screens can talk to.
#[tauri::command]
fn bmw_e70_modules() -> Vec<ModuleInfo> {
    bmw_e70::MODULES
        .iter()
        .map(|m| ModuleInfo {
            name: m.name,
            description: m.description,
            address: m.address,
            response_id: m.response_id(),
        })
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_devices, bmw_e70_modules])
        .run(tauri::generate_context!())
        .expect("error while running ApexDiag");
}
