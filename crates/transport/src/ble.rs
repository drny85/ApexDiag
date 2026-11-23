//! Bluetooth Low Energy link to ELM327-style adapters.
//!
//! Built on `btleplug`, which drives CoreBluetooth on both macOS and iOS
//! (iPadOS), so the same code reaches a Veepeak OBDCheck BLE from a Mac or an
//! iPad. BLE OBD adapters expose a UART-like GATT service: the app writes AT
//! and OBD commands to one characteristic and receives the replies as
//! notifications on another. The common layouts are:
//!
//! | Adapter                       | Service | Write | Notify |
//! |-------------------------------|---------|-------|--------|
//! | Veepeak OBDCheck BLE, most clones | FFF0 | FFF2 | FFF1 |
//! | vLinker, Konnwei, many clones | FFE0    | FFE1  | FFE1   |
//! | OBDLink CX                    | vendor 128-bit, found by properties |
//!
//! When no known pair is present, the first characteristic that can notify
//! and the first that can be written (in the same service) are used.

use crate::{DiscoveredDevice, HardwareTransport, Result, TransportError, TransportKind};
use async_trait::async_trait;
use btleplug::api::{
    Central, CharPropFlags, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

/// Prefix of [`DiscoveredDevice::id`] for adapters found over BLE.
pub const ID_PREFIX: &str = "ble:";

/// BLE writes larger than this are split. 20 bytes is the payload of the
/// smallest ATT MTU every adapter accepts; OBD command lines are shorter.
const CHUNK: usize = 20;

const fn uuid16(short: u16) -> Uuid {
    Uuid::from_u128(((short as u128) << 96) | 0x0000_0000_0000_1000_8000_0080_5f9b_34fb)
}

/// (write, notify) pairs we recognise, in order of preference.
const KNOWN_PAIRS: &[(Uuid, Uuid)] = &[
    (uuid16(0xFFF2), uuid16(0xFFF1)),
    (uuid16(0xFFE1), uuid16(0xFFE1)),
    (uuid16(0xFFE2), uuid16(0xFFE1)),
    (uuid16(0x2AF1), uuid16(0x2AF0)),
];

/// Names that identify an OBD adapter during a scan.
const OBD_NAME_HINTS: &[&str] = &[
    "OBD", "VEEPEAK", "VLINK", "V-LINK", "ELM", "KONNWEI", "OBDII", "OBDLINK", "LELINK",
    "CARISTA", "VGATE", "ICAR",
];

struct Registry {
    adapter: Adapter,
    found: HashMap<String, Peripheral>,
}

/// One CoreBluetooth central for the app's lifetime.
fn registry() -> &'static Mutex<Option<Registry>> {
    static REG: OnceLock<Mutex<Option<Registry>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(None))
}

fn other(e: impl std::fmt::Display) -> TransportError {
    TransportError::Other(e.to_string())
}

async fn ensure_adapter(slot: &mut Option<Registry>) -> Result<&mut Registry> {
    if slot.is_none() {
        let manager = Manager::new().await.map_err(other)?;
        let adapter = manager
            .adapters()
            .await
            .map_err(other)?
            .into_iter()
            .next()
            .ok_or_else(|| other("No Bluetooth radio found. Turn Bluetooth on and try again."))?;
        *slot = Some(Registry {
            adapter,
            found: HashMap::new(),
        });
    }
    Ok(slot.as_mut().expect("set above"))
}

fn looks_like_obd(name: &str) -> bool {
    let up = name.to_ascii_uppercase();
    OBD_NAME_HINTS.iter().any(|h| up.contains(h))
}

/// Scans for `window` and returns the adapters seen. Devices whose names look
/// like OBD adapters come first; with `all` false, only those are returned.
pub async fn scan(window: Duration, all: bool) -> Result<Vec<DiscoveredDevice>> {
    let mut slot = registry().lock().await;
    let reg = ensure_adapter(&mut slot).await?;
    // CoreBluetooth reports "powered on" a moment after the central is made,
    // and the first launch on iOS waits for the permission prompt. Retry the
    // scan start for a few seconds before giving up.
    let mut started = false;
    for _ in 0..20 {
        if reg.adapter.start_scan(ScanFilter::default()).await.is_ok() {
            started = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    if !started {
        return Err(other(
            "Bluetooth is off or ApexDiag is not allowed to use it. Check Settings > Privacy > Bluetooth.",
        ));
    }
    tokio::time::sleep(window).await;
    let _ = reg.adapter.stop_scan().await;

    let mut out = Vec::new();
    for p in reg.adapter.peripherals().await.map_err(other)? {
        let Ok(Some(props)) = p.properties().await else {
            continue;
        };
        let name = props.local_name.clone().unwrap_or_default();
        let obd = looks_like_obd(&name);
        if name.is_empty() || (!all && !obd) {
            continue;
        }
        let pid = p.id().to_string();
        let rssi = props
            .rssi
            .map(|r| format!("signal {r} dBm"))
            .unwrap_or_else(|| "signal unknown".into());
        out.push((
            obd,
            props.rssi.unwrap_or(i16::MIN),
            DiscoveredDevice {
                id: format!("{ID_PREFIX}{pid}"),
                name,
                detail: format!("Bluetooth LE adapter, {rssi}."),
                transport: TransportKind::Ble {
                    peripheral_id: pid.clone(),
                },
                available: true,
            },
        ));
        reg.found.insert(pid, p);
    }
    out.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    Ok(out.into_iter().map(|(_, _, d)| d).collect())
}

/// Picks the write and notify characteristics of a BLE serial adapter.
fn pick_chars(chars: &[Characteristic]) -> Option<(Characteristic, Characteristic)> {
    let find = |u: Uuid| chars.iter().find(|c| c.uuid == u);
    for (w, n) in KNOWN_PAIRS {
        if let (Some(w), Some(n)) = (find(*w), find(*n)) {
            let can_write = w
                .properties
                .intersects(CharPropFlags::WRITE | CharPropFlags::WRITE_WITHOUT_RESPONSE);
            let can_notify = n
                .properties
                .intersects(CharPropFlags::NOTIFY | CharPropFlags::INDICATE);
            if can_write && can_notify {
                return Some((w.clone(), n.clone()));
            }
        }
    }
    // Fallback: a service holding both a notifiable and a writable
    // characteristic. Skip the standard GATT services (0x1800-0x18FF).
    let standard = |u: &Uuid| {
        let v = u.as_u128();
        (v & 0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF) == 0x0000_0000_1000_8000_0080_5f9b_34fb
            && (0x1800..0x1900).contains(&((v >> 96) as u32))
    };
    for n in chars.iter().filter(|c| {
        !standard(&c.service_uuid)
            && c.properties
                .intersects(CharPropFlags::NOTIFY | CharPropFlags::INDICATE)
    }) {
        if let Some(w) = chars.iter().find(|c| {
            c.service_uuid == n.service_uuid
                && c.properties
                    .intersects(CharPropFlags::WRITE | CharPropFlags::WRITE_WITHOUT_RESPONSE)
        }) {
            return Some((w.clone(), n.clone()));
        }
    }
    None
}

/// A connected (or connectable) BLE adapter.
pub struct BleTransport {
    peripheral_id: String,
    peripheral: Option<Peripheral>,
    write: Option<Characteristic>,
    write_type: WriteType,
    rx: Option<mpsc::UnboundedReceiver<Vec<u8>>>,
    pump: Option<tokio::task::JoinHandle<()>>,
}

impl BleTransport {
    /// A link to a peripheral found by [`scan`].
    pub fn new(peripheral_id: impl Into<String>) -> Self {
        Self {
            peripheral_id: peripheral_id.into(),
            peripheral: None,
            write: None,
            write_type: WriteType::WithoutResponse,
            rx: None,
            pump: None,
        }
    }
}

#[async_trait]
impl HardwareTransport for BleTransport {
    async fn connect(&mut self) -> Result<()> {
        let p = {
            let slot = registry().lock().await;
            slot.as_ref()
                .and_then(|r| r.found.get(&self.peripheral_id).cloned())
                .ok_or_else(|| other("Adapter not found. Scan again with the adapter plugged in."))?
        };
        let attempt = async {
            if !p.is_connected().await.unwrap_or(false) {
                p.connect().await.map_err(other)?;
            }
            p.discover_services().await.map_err(other)?;
            Ok::<_, TransportError>(())
        };
        tokio::time::timeout(Duration::from_secs(12), attempt)
            .await
            .map_err(|_| TransportError::Timeout(Duration::from_secs(12)))??;

        let chars: Vec<Characteristic> = p.characteristics().into_iter().collect();
        let (w, n) = pick_chars(&chars).ok_or_else(|| {
            other("This Bluetooth device has no serial data channel. Is it an OBD adapter?")
        })?;
        self.write_type = if w.properties.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE) {
            WriteType::WithoutResponse
        } else {
            WriteType::WithResponse
        };
        p.subscribe(&n).await.map_err(other)?;
        let mut stream = p.notifications().await.map_err(other)?;
        let (tx, rx) = mpsc::unbounded_channel();
        let notify_uuid = n.uuid;
        self.pump = Some(tokio::spawn(async move {
            while let Some(v) = stream.next().await {
                if v.uuid == notify_uuid && tx.send(v.value).is_err() {
                    break;
                }
            }
        }));
        self.rx = Some(rx);
        self.write = Some(w);
        self.peripheral = Some(p);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(h) = self.pump.take() {
            h.abort();
        }
        self.rx = None;
        self.write = None;
        if let Some(p) = self.peripheral.take() {
            let _ = p.disconnect().await;
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.peripheral.is_some() && self.rx.is_some()
    }

    async fn send(&mut self, bytes: &[u8]) -> Result<()> {
        let (Some(p), Some(w)) = (&self.peripheral, &self.write) else {
            return Err(TransportError::NotConnected);
        };
        // Drop stale bytes from an earlier, timed-out exchange so they are
        // not read as the answer to this one.
        if let Some(rx) = self.rx.as_mut() {
            while rx.try_recv().is_ok() {}
        }
        for chunk in bytes.chunks(CHUNK) {
            p.write(w, chunk, self.write_type).await.map_err(other)?;
        }
        Ok(())
    }

    async fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let rx = self.rx.as_mut().ok_or(TransportError::NotConnected)?;
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(mut first)) => {
                while let Ok(more) = rx.try_recv() {
                    first.extend_from_slice(&more);
                }
                Ok(first)
            }
            Ok(None) => Err(other("Bluetooth adapter disconnected.")),
            Err(_) => Err(TransportError::Timeout(timeout)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btleplug::api::CharPropFlags as F;
    use std::collections::BTreeSet;

    fn ch(service: u16, uuid: Uuid, props: F) -> Characteristic {
        Characteristic {
            uuid,
            service_uuid: uuid16(service),
            properties: props,
            descriptors: BTreeSet::new(),
        }
    }

    #[test]
    fn veepeak_layout_picks_fff2_and_fff1() {
        let chars = vec![
            ch(0x180A, uuid16(0x2A29), F::READ),
            ch(0xFFF0, uuid16(0xFFF1), F::NOTIFY),
            ch(0xFFF0, uuid16(0xFFF2), F::WRITE_WITHOUT_RESPONSE | F::WRITE),
        ];
        let (w, n) = pick_chars(&chars).unwrap();
        assert_eq!(w.uuid, uuid16(0xFFF2));
        assert_eq!(n.uuid, uuid16(0xFFF1));
    }

    #[test]
    fn vlinker_layout_shares_one_characteristic() {
        let chars = vec![ch(
            0xFFE0,
            uuid16(0xFFE1),
            F::NOTIFY | F::WRITE_WITHOUT_RESPONSE,
        )];
        let (w, n) = pick_chars(&chars).unwrap();
        assert_eq!(w.uuid, n.uuid);
    }

    #[test]
    fn unknown_vendor_service_found_by_properties() {
        let svc = Uuid::from_u128(0xe7810a71_73ae_499d_8c15_faa9aef0c3f2);
        let c = Uuid::from_u128(0xbef8d6c9_9c21_4c9e_b632_bd58c1009f9f);
        let chars = vec![
            ch(0x1800, uuid16(0x2A00), F::READ | F::WRITE),
            Characteristic {
                uuid: c,
                service_uuid: svc,
                properties: F::NOTIFY | F::WRITE,
                descriptors: BTreeSet::new(),
            },
        ];
        let (w, _) = pick_chars(&chars).unwrap();
        assert_eq!(w.uuid, c);
    }

    #[test]
    fn names() {
        assert!(looks_like_obd("OBDII"));
        assert!(looks_like_obd("VEEPEAK"));
        assert!(looks_like_obd("IOS-Vlink"));
        assert!(!looks_like_obd("Robert's AirPods"));
    }
}
