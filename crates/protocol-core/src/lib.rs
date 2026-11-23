//! Protocol engine for ApexDiag.
//!
//! Everything in this crate is synchronous, allocation-light and free of I/O:
//! each module is a state machine that turns bytes in into bytes out. The
//! `transport` crate moves those bytes over BLE, serial or TCP.

pub mod bmw_e70;
pub mod catalog;
pub mod copilot;
pub mod diag;
pub mod doip;
pub mod elm327;
pub mod hsfz;
pub mod isotp;
pub mod obd2;
pub mod packs;
pub mod session;
pub mod snapshot;
pub mod vehicle;
pub mod vin;

/// A CAN identifier (11-bit for D-CAN / J1979 on the E70).
pub type CanId = u32;

/// Errors produced by the protocol state machines.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("payload too long for ISO-TP: {0} bytes")]
    PayloadTooLong(usize),
    #[error("malformed frame: {0}")]
    MalformedFrame(&'static str),
    #[error("unexpected frame type for current state")]
    UnexpectedFrame,
    #[error("negative response 0x{service:02X}, NRC 0x{nrc:02X}")]
    NegativeResponse { service: u8, nrc: u8 },
    #[error("not implemented yet")]
    NotImplemented,
}
