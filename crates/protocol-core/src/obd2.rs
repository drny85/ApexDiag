//! SAE J1979 generic OBD-II services (functional broadcast on 0x7DF).
//!
//! Step 2 adds PID decoding, DTC parsing and VIN reassembly.

use crate::CanId;

/// Functional (broadcast) request ID for 11-bit OBD-II.
pub const FUNCTIONAL_REQUEST: CanId = 0x7DF;

/// J1979 service identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Service {
    CurrentData = 0x01,
    StoredDtcs = 0x03,
    ClearDtcs = 0x04,
    PendingDtcs = 0x07,
    PermanentDtcs = 0x0A,
    VehicleInfo = 0x09,
}

/// A few common Service 0x01 PIDs.
pub mod pid {
    pub const COOLANT_TEMP: u8 = 0x05;
    pub const SHORT_FUEL_TRIM_B1: u8 = 0x06;
    pub const LONG_FUEL_TRIM_B1: u8 = 0x07;
    pub const MAP: u8 = 0x0B;
    pub const RPM: u8 = 0x0C;
    pub const SPEED: u8 = 0x0D;
    pub const TIMING_ADVANCE: u8 = 0x0E;
}
