//! BMW E70 X5 (2007-2013) modules on D-CAN, spoken to with KWP2000.
//!
//! The tester sends on CAN ID `0x6F1` with the target module address as the
//! first data byte (extended addressing). Each module replies on
//! `0x600 + address` with `0xF1` (the tester) as its first data byte.
//!
//! The address list below is the commonly documented E-series map. Which
//! modules are actually fitted depends on the car's options, so the app
//! always confirms by scanning (see [`scan_request`]) and marks modules
//! from this list as "expected" until they answer.

use crate::diag::Protocol;
use crate::CanId;

/// Tester request CAN ID used for all physical requests.
pub const TESTER_ID: CanId = 0x6F1;
/// Tester address, the first data byte of every module reply.
pub const TESTER_ADDRESS: u8 = 0xF1;

/// What part of the car a module belongs to, for grouping in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum System {
    Powertrain,
    Chassis,
    Body,
    Comfort,
    Infotainment,
    Safety,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Module {
    /// BMW short name as used in BMW documentation (DME, EGS, ...).
    pub name: &'static str,
    /// What it is, in plain words.
    pub description: &'static str,
    /// What it controls and why you'd talk to it.
    pub role: &'static str,
    pub address: u8,
    pub system: System,
}

impl Module {
    /// CAN ID the module answers on.
    pub const fn response_id(&self) -> CanId {
        0x600 + self.address as CanId
    }

    /// E70 modules speak KWP2000 on D-CAN.
    pub const fn protocol(&self) -> Protocol {
        Protocol::Kwp2000
    }
}

pub const JBE: Module = Module {
    name: "JBE",
    description: "Junction box electronics",
    role: "Central power distribution and gateway for body functions: wipers, interior lights, fuel pump relay, and many fuses.",
    address: 0x00,
    system: System::Body,
};
pub const ACSM: Module = Module {
    name: "ACSM",
    description: "Airbag / crash safety module",
    role: "Controls airbags and seat-belt pretensioners. Read-only in this app apart from clearing stored crash-free faults.",
    address: 0x01,
    system: System::Safety,
};
pub const DME: Module = Module {
    name: "DME",
    description: "Engine control (petrol)",
    role: "Runs the engine: fuel injection, ignition, VANOS, throttle, electric water pump, and emissions monitors.",
    address: 0x12,
    system: System::Powertrain,
};
pub const EGS: Module = Module {
    name: "EGS",
    description: "Automatic transmission",
    role: "Controls the 6-speed automatic: shift timing, torque converter lock-up and clutch pressures.",
    address: 0x18,
    system: System::Powertrain,
};
pub const VGSG: Module = Module {
    name: "VGSG",
    description: "Transfer case (xDrive)",
    role: "Splits torque between front and rear axles via the transfer case clutch motor.",
    address: 0x19,
    system: System::Powertrain,
};
pub const DSC: Module = Module {
    name: "DSC",
    description: "ABS / stability control",
    role: "Anti-lock brakes, traction and stability control, hill descent, and the wheel speed sensors.",
    address: 0x29,
    system: System::Chassis,
};
pub const CAS: Module = Module {
    name: "CAS",
    description: "Car access system",
    role: "Keys, immobilizer, start/stop button and central locking authorisation. Also stores the car's mileage and VIN copy.",
    address: 0x40,
    system: System::Body,
};
pub const FZD: Module = Module {
    name: "FZD",
    description: "Roof function centre",
    role: "Sunroof, interior lights in the headliner, and the anti-theft interior movement sensor.",
    address: 0x56,
    system: System::Comfort,
};
pub const KOMBI: Module = Module {
    name: "KOMBI",
    description: "Instrument cluster",
    role: "Gauges, warning lights, check-control messages and service intervals.",
    address: 0x60,
    system: System::Body,
};
pub const HEAD_UNIT: Module = Module {
    name: "CCC/CIC",
    description: "Head unit (iDrive)",
    role: "Navigation, radio, iDrive screen and vehicle settings menus.",
    address: 0x63,
    system: System::Infotainment,
};
pub const PDC: Module = Module {
    name: "PDC",
    description: "Park distance control",
    role: "Parking sensors front and rear.",
    address: 0x64,
    system: System::Comfort,
};
pub const FRM: Module = Module {
    name: "FRM",
    description: "Footwell module",
    role:
        "Exterior lights, window regulators and the light switch. Bulb-out warnings come from here.",
    address: 0x72,
    system: System::Body,
};
pub const IHKA: Module = Module {
    name: "IHKA",
    description: "Climate control",
    role: "Heating and air conditioning: blower, flaps, A/C compressor request and temperature sensors.",
    address: 0x78,
    system: System::Comfort,
};

/// Commonly documented E70 modules. Confirm on the car with a scan.
pub const MODULES: &[Module] = &[
    JBE, ACSM, DME, EGS, VGSG, DSC, CAS, FZD, KOMBI, HEAD_UNIT, PDC, FRM, IHKA,
];

/// Finds a known module by address.
pub fn module_at(address: u8) -> Option<&'static Module> {
    MODULES.iter().find(|m| m.address == address)
}

/// Request used to discover whether a module is present: TesterPresent with
/// a reply requested. Any reply, positive or negative, means "fitted".
pub const fn scan_request() -> [u8; 2] {
    [0x3E, 0x01]
}

/// Status of a special-function definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Payload confirmed against a real E70.
    Verified,
    /// We know what the job does but not yet the exact request bytes for
    /// this car. The app refuses to run it until it is verified.
    PendingCapture,
}

/// A BMW service job: which module, what it sends, and how sure we are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Job {
    /// Catalog id, matches [`crate::catalog`].
    pub function_id: &'static str,
    /// BMW job name, for cross-reference with BMW documentation, where we
    /// are sure of it.
    pub bmw_job: Option<&'static str>,
    pub module: Module,
    /// Request bytes, if known.
    pub request: Option<&'static [u8]>,
    pub status: JobStatus,
}

/// Special functions targeted for the E70. Requests are filled in once they
/// are confirmed on Robert's car; until then the app shows the job but won't
/// send anything.
pub const JOBS: &[Job] = &[
    Job {
        function_id: "coolant_bleed",
        bmw_job: Some("STEUERN_KUEHLMITTELPUMPE"),
        module: DME,
        request: None,
        status: JobStatus::PendingCapture,
    },
    Job {
        function_id: "battery_registration",
        bmw_job: None,
        module: DME,
        request: None,
        status: JobStatus::PendingCapture,
    },
    Job {
        function_id: "dme_adaptation_reset",
        bmw_job: None,
        module: DME,
        request: None,
        status: JobStatus::PendingCapture,
    },
    Job {
        function_id: "egs_adaptation_reset",
        bmw_job: None,
        module: EGS,
        request: None,
        status: JobStatus::PendingCapture,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_ids() {
        assert_eq!(DME.response_id(), 0x612);
        assert_eq!(EGS.response_id(), 0x618);
        assert_eq!(DSC.response_id(), 0x629);
        assert_eq!(CAS.response_id(), 0x640);
    }

    #[test]
    fn addresses_are_unique() {
        for (i, a) in MODULES.iter().enumerate() {
            for b in &MODULES[i + 1..] {
                assert_ne!(a.address, b.address, "{} and {}", a.name, b.name);
            }
        }
        assert_eq!(module_at(0x12), Some(&DME));
    }

    #[test]
    fn unverified_jobs_carry_no_payload() {
        for job in JOBS {
            assert_eq!(
                job.status == JobStatus::Verified,
                job.request.is_some(),
                "{}",
                job.function_id
            );
        }
    }
}
