//! Vehicle packs beyond the BMW E70 (see [`crate::bmw_e70`]).
//!
//! These module maps are demo layouts: names, roles and locations follow the
//! brands' own conventions, but the CAN IDs and record identifiers are not
//! yet confirmed on a real car. On a real car the app only scans the
//! legislated OBD-II engine and transmission IDs of a demo layout, and never
//! writes with an unverified identifier.

pub mod honda_accord;
pub mod jeep_jl;
pub mod mercedes_w167;
pub mod toyota_xv70;

use crate::diag::Protocol;
use crate::snapshot::{Format, RecordDef, RecordKind};
use crate::vehicle::{Job, JobStatus, Link, Module, Spot, System};

pub(crate) const OFF_ON: &[(u8, &str)] = &[(0x00, "Off"), (0x01, "On")];

/// A UDS module on 11-bit CAN.
pub(crate) const fn uds(
    name: &'static str,
    description: &'static str,
    role: &'static str,
    tx: u16,
    rx: u16,
    system: System,
    spot: (u16, u16, &'static str),
) -> Module {
    Module {
        name,
        description,
        role,
        address: (tx & 0xFF) as u8,
        system,
        link: Link::Can11 { tx, rx },
        protocol: Protocol::Uds,
        spot: Spot {
            x: spot.0,
            y: spot.1,
            area: spot.2,
        },
    }
}

pub(crate) const fn num(
    bytes: u8,
    signed: bool,
    scale: f64,
    unit: &'static str,
    digits: u8,
) -> Format {
    Format::Number {
        bytes,
        signed,
        scale,
        unit,
        digits,
    }
}

pub(crate) const fn rec(
    module: &Module,
    id: u16,
    name: &'static str,
    kind: RecordKind,
    format: Format,
    explain: &'static str,
) -> RecordDef {
    RecordDef {
        module: module.address,
        id,
        name,
        kind,
        format,
        explain,
        status: JobStatus::PendingCapture,
    }
}

pub(crate) const fn job(function_id: &'static str, module: &Module) -> Job {
    Job {
        function_id,
        oem_job: None,
        module: module.address,
        request: None,
        status: JobStatus::PendingCapture,
    }
}
