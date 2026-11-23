# ApexDiag

Native macOS automotive diagnostic suite: generic OBD-II (SAE J1979) for any car, plus
manufacturer-level diagnostics for the BMW E70 X5 (KWP2000 over D-CAN) and, later, F/G
chassis (UDS over DoIP).

## Layout

| Path | What it is |
| --- | --- |
| `crates/protocol-core` | Pure-Rust protocol state machines: ISO-TP, ELM327 commands, J1979 services, BMW E70 address map. No I/O. |
| `crates/transport` | `HardwareTransport` trait plus adapters (BLE, serial, DoIP). Ships a scripted `MockTransport` for tests. |
| `src-tauri` | Tauri v2 shell that exposes the Rust core to the UI as commands. |
| `src` | React 19 + TypeScript + Tailwind v4 + shadcn/ui frontend. |

## Requirements (macOS, Apple Silicon)

- Xcode Command Line Tools: `xcode-select --install`
- Rust: https://rustup.rs
- Bun: https://bun.sh

## Run it

```sh
bun install
bun tauri dev               # desktop app with hot reload
bun run dev                 # UI only, in a browser (uses stub data)
cargo test --workspace      # Rust tests
bun tauri build --target aarch64-apple-darwin   # .app and .dmg
```

## Supported adapters

| Adapter | Link | Status |
| --- | --- | --- |
| ELM327-compatible BLE (e.g. Veepeak OBDCheck BLE) | Bluetooth LE | Planned, step 3 |
| OBDLink / STN11xx | BLE or Bluetooth serial | Planned |
| K+DCAN USB (FTDI) | USB serial | Later |
| BMW ENET | DoIP, TCP 13400 | Later |

## Safety

Actuator tests and service routines will only run when the vehicle is stationary and after a
two-step confirmation. Use at your own risk; this is not a dealer tool.
