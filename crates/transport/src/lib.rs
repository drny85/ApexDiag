//! Byte-level links to diagnostic adapters.
//!
//! Every adapter, whether an OBDLink over Bluetooth, a K+DCAN USB cable or a
//! DoIP gateway, is exposed through [`HardwareTransport`]. Protocol code above
//! this layer never knows which one it is talking to.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub mod mock;

/// How the app reaches the adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportKind {
    /// Bluetooth Low Energy (OBDLink CX, vLinker BLE, ...).
    Ble { peripheral_id: String },
    /// A serial device: Bluetooth Classic SPP (OBDLink MX+) or USB (K+DCAN).
    Serial { path: String, baud: u32 },
    /// Diagnostics over IP for F/G chassis, TCP port 13400.
    Doip { host: String, port: u16 },
    /// In-memory adapter for tests and UI development.
    Mock,
}

/// An adapter the user can pick in the connection manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub name: String,
    pub transport: TransportKind,
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
}
