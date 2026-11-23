//! Byte-level links to diagnostic adapters.
//!
//! Every adapter, whether an OBDLink over Bluetooth, a K+DCAN USB cable or a
//! DoIP gateway, is exposed through [`HardwareTransport`]. Protocol code above
//! this layer never knows which one it is talking to.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(feature = "ble")]
pub mod ble;
pub mod capture;
pub mod channel;
pub mod elm;
pub mod full_scan;
pub mod mock;
pub mod sim;

/// How the app reaches the adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportKind {
    /// Bluetooth Low Energy (Veepeak OBDCheck BLE, OBDLink CX, vLinker, ...).
    Ble { peripheral_id: String },
    /// A serial device: Bluetooth Classic SPP (OBDLink MX+) or USB (K+DCAN).
    Serial { path: String, baud: u32 },
    /// Diagnostics over IP for F/G chassis, TCP port 13400.
    Doip { host: String, port: u16 },
    /// The built-in simulated E70 X5, for demos and development.
    Simulator { engine_running: bool },
    /// In-memory adapter for tests.
    Mock,
}

/// An adapter the user can pick in the connection manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub transport: TransportKind,
    /// False for adapters listed for information but not usable yet.
    pub available: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("not connected")]
    NotConnected,
    #[error("timed out after {0:?}")]
    Timeout(Duration),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, TransportError>;

/// A bidirectional byte link to a diagnostic adapter.
#[async_trait]
pub trait HardwareTransport: Send {
    /// Open the link.
    async fn connect(&mut self) -> Result<()>;
    /// Close the link. Safe to call when already closed.
    async fn disconnect(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;
    /// Write raw bytes to the adapter.
    async fn send(&mut self, bytes: &[u8]) -> Result<()>;
    /// Read whatever bytes arrive before `timeout` expires.
    async fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>>;
    /// The simulator behind this link, if it is one. Lets the demo change
    /// the simulated car's state.
    fn simulator(&mut self) -> Option<&mut sim::ElmSimulator> {
        None
    }
    /// Adds a note to the raw capture, if this link is being captured.
    fn capture_note(&mut self, _text: &str) {}
}

#[async_trait]
impl HardwareTransport for Box<dyn HardwareTransport + Send> {
    async fn connect(&mut self) -> Result<()> {
        (**self).connect().await
    }
    async fn disconnect(&mut self) -> Result<()> {
        (**self).disconnect().await
    }
    fn is_connected(&self) -> bool {
        (**self).is_connected()
    }
    async fn send(&mut self, bytes: &[u8]) -> Result<()> {
        (**self).send(bytes).await
    }
    async fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        (**self).recv(timeout).await
    }
    fn simulator(&mut self) -> Option<&mut sim::ElmSimulator> {
        (**self).simulator()
    }
    fn capture_note(&mut self, text: &str) {
        (**self).capture_note(text)
    }
}
