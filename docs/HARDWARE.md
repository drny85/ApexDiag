# Interfaces: what works today and what comes next

ApexDiag talks to cars through two kinds of link:

- **Byte links** (`transport::HardwareTransport`): an ELM327-style adapter that we drive with text commands. The ELM client turns KWP2000/UDS requests into AT commands and CAN frames.
- **Message channels** (`transport::channel::DiagChannel`): a professional interface that takes a whole request for one module and returns the whole reply. ELM327 implements it too, so module features can be written once against `DiagChannel`.

| Interface | Cars | Link | Status |
|---|---|---|---|
| Veepeak OBDCheck BLE, vLinker, OBDLink CX (ELM327 over BLE) | Any OBD-II car; BMW E-series modules through `AT CEA` | `transport::ble` on macOS, iPadOS, Windows | **Working in the app.** First real-car test pending. |
| Practice cars | Five simulated cars | `transport::sim` | Working |
| BMW ENET cable (HSFZ, TCP 6801) | BMW F and G series | `channel::HsfzChannel` + `protocol_core::hsfz` | **Groundwork.** Codec and channel tested against a fake gateway. Needs: gateway discovery (UDP 6811), a Connect-page entry, and a run on a real F/G car. |
| DoIP (ISO 13400, TCP 13400) | BMW G series, newer Mercedes-Benz, VW group and others | `channel::DoipChannel` + `protocol_core::doip` | **Groundwork.** Routing activation and diagnostic messages tested against a fake gateway. Needs: a module-to-logical-address map for each maker, vehicle discovery, and TLS for cars that require it. |
| K+DCAN USB cable | BMW E-series (2007 to about 2013 on D-CAN, older on K-line) | Serial (FTDI) | Not started. Works on Mac and Windows; iPad can't use USB serial. |
| J2534 pass-thru (Mongoose, Drew Tech, Autel and others) | Most makes, dealer-level | Vendor DLL | Not started. **Windows only**: J2534 is a Windows DLL API. |
| OBDLink MX+ (Bluetooth Classic) | Any OBD-II car | Serial over Bluetooth | Not started. Faster than ELM327 clones. macOS and Windows only: iPadOS doesn't allow Bluetooth Classic serial. |

## Platforms

- **macOS (Apple Silicon)**: the main build.
- **iPadOS**: BLE adapters only (Apple allows BLE and network; USB serial and Bluetooth Classic serial are closed). ENET and DoIP can work through a USB-C Ethernet adapter, the same as the Mac.
- **Windows**: CI builds the app and runs the protocol and transport tests on every pull request (`windows-app` in `.github/workflows/ci.yml`). Bluetooth LE goes through WinRT via btleplug. J2534 support would live here.

## Why this order

ELM327 over BLE reaches every OBD-II car and most BMW E-series modules, at the lowest hardware cost. ENET and DoIP are what newer BMWs and other premium makes need for dealer-level work. J2534 is what shops already own, which matters when the app is sold to them.
