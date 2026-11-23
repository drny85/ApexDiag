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
use crate::vehicle::{BodyStyle, Layout, Link, Make, Pack, Spot};
use crate::CanId;

pub use crate::vehicle::{Job, JobStatus, Module, System};

/// Tester request CAN ID used for all physical requests.
pub const TESTER_ID: CanId = crate::vehicle::BMW_TESTER_ID;
/// Tester address, the first data byte of every module reply.
pub const TESTER_ADDRESS: u8 = crate::vehicle::BMW_TESTER_ADDRESS;

const fn spot(x: u16, y: u16, area: &'static str) -> Spot {
    Spot { x, y, area }
}

pub const JBE: Module = Module {
    name: "JBE",
    description: "Junction box electronics",
    role: "Central power distribution and gateway for body functions: wipers, interior lights, fuel pump relay, and many fuses.",
    address: 0x00,
    system: System::Body,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(182, 204, "Passenger footwell"),
};
pub const ACSM: Module = Module {
    name: "ACSM",
    description: "Airbag / crash safety module",
    role: "Controls airbags and seat-belt pretensioners. Read-only in this app apart from clearing stored crash-free faults.",
    address: 0x01,
    system: System::Safety,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(148, 268, "Under the centre console"),
};
pub const DME: Module = Module {
    name: "DME",
    description: "Engine control (petrol)",
    role: "Runs the engine: fuel injection, ignition, VANOS, throttle, electric water pump, and emissions monitors.",
    address: 0x12,
    system: System::Powertrain,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(166, 120, "Engine bay, passenger side, in the electronics box"),
};
pub const EGS: Module = Module {
    name: "EGS",
    description: "Automatic transmission",
    role: "Controls the 6-speed automatic: shift timing, torque converter lock-up and clutch pressures.",
    address: 0x18,
    system: System::Powertrain,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(120, 256, "On the transmission, under the centre tunnel"),
};
pub const VGSG: Module = Module {
    name: "VGSG",
    description: "Transfer case (xDrive)",
    role: "Splits torque between front and rear axles via the transfer case clutch motor.",
    address: 0x19,
    system: System::Powertrain,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(120, 290, "Transfer case, under the car"),
};
pub const DSC: Module = Module {
    name: "DSC",
    description: "ABS / stability control",
    role: "Anti-lock brakes, traction and stability control, hill descent, and the wheel speed sensors.",
    address: 0x29,
    system: System::Chassis,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(74, 78, "Engine bay, driver side, front"),
};
pub const CAS: Module = Module {
    name: "CAS",
    description: "Car access system",
    role: "Keys, immobilizer, start/stop button and central locking authorisation. Also stores the car's mileage and VIN copy.",
    address: 0x40,
    system: System::Body,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(100, 198, "Under the dash, left of the steering column"),
};
pub const FZD: Module = Module {
    name: "FZD",
    description: "Roof function centre",
    role: "Sunroof, interior lights in the headliner, and the anti-theft interior movement sensor.",
    address: 0x56,
    system: System::Comfort,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(120, 226, "Roof console, above the mirror"),
};
pub const KOMBI: Module = Module {
    name: "KOMBI",
    description: "Instrument cluster",
    role: "Gauges, warning lights, check-control messages and service intervals.",
    address: 0x60,
    system: System::Body,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(88, 178, "Behind the instruments"),
};
pub const HEAD_UNIT: Module = Module {
    name: "CCC/CIC",
    description: "Head unit (iDrive)",
    role: "Navigation, radio, iDrive screen and vehicle settings menus.",
    address: 0x63,
    system: System::Infotainment,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(152, 178, "Dashboard, behind the centre display"),
};
pub const PDC: Module = Module {
    name: "PDC",
    description: "Park distance control",
    role: "Parking sensors front and rear.",
    address: 0x64,
    system: System::Comfort,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(178, 424, "Rear, behind the boot trim"),
};
pub const FRM: Module = Module {
    name: "FRM",
    description: "Footwell module",
    role:
        "Exterior lights, window regulators and the light switch. Bulb-out warnings come from here.",
    address: 0x72,
    system: System::Body,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(58, 204, "Driver footwell, left kick panel"),
};
pub const IHKA: Module = Module {
    name: "IHKA",
    description: "Climate control",
    role: "Heating and air conditioning: blower, flaps, A/C compressor request and temperature sensors.",
    address: 0x78,
    system: System::Comfort,
    link: Link::BmwExtended,
    protocol: Protocol::Kwp2000,
    spot: spot(120, 182, "Centre of the dashboard"),
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

/// Special functions targeted for the E70. Requests are filled in once they
/// are confirmed on Robert's car; until then the app shows the job but won't
/// send anything.
pub const JOBS: &[Job] = &[
    Job {
        function_id: "coolant_bleed",
        oem_job: Some("STEUERN_KUEHLMITTELPUMPE"),
        module: DME.address,
        request: None,
        status: JobStatus::PendingCapture,
    },
    Job {
        function_id: "battery_registration",
        oem_job: None,
        module: DME.address,
        request: None,
        status: JobStatus::PendingCapture,
    },
    Job {
        function_id: "dme_adaptation_reset",
        oem_job: None,
        module: DME.address,
        request: None,
        status: JobStatus::PendingCapture,
    },
    Job {
        function_id: "egs_adaptation_reset",
        oem_job: None,
        module: EGS.address,
        request: None,
        status: JobStatus::PendingCapture,
    },
];

// ---- Time Machine records ---------------------------------------------

use crate::snapshot::{Format, RecordDef, RecordKind};

const OFF_ON: &[(u8, &str)] = &[(0x00, "Off"), (0x01, "On")];

const fn num(bytes: u8, signed: bool, scale: f64, unit: &'static str, digits: u8) -> Format {
    Format::Number {
        bytes,
        signed,
        scale,
        unit,
        digits,
    }
}

const fn rec(
    module: Module,
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

use RecordKind::{Adaptation, Coding};

/// Records the Time Machine reads from each module.
///
/// These identifiers are a demo layout used by the simulated car. None are
/// confirmed on a real E70 yet, so on a real car the Time Machine reads only
/// identification and fault memory until each record is verified.
pub const RECORDS: &[RecordDef] = &[
    // DME: engine
    rec(DME, 0x01, "Mixture adaptation, bank 1", Adaptation, num(2, true, 0.01, "%", 2),
        "How much extra (+) or less (-) fuel the engine has learned to add on cylinders 1-3. Near 0 is healthy. A steady climb usually means unmetered air: a vacuum leak, a cracked crankcase vent valve, or a weak fuel pump."),
    rec(DME, 0x02, "Mixture adaptation, bank 2", Adaptation, num(2, true, 0.01, "%", 2),
        "The same learned fuel correction for cylinders 4-6. When both banks drift together the cause is shared, like a vacuum leak; when only one drifts, look at that bank's injectors or intake gasket."),
    rec(DME, 0x03, "Idle speed correction", Adaptation, num(2, true, 1.0, "rpm", 0),
        "How much the engine has learned to raise idle to stay smooth. Creeping upward points to a dirty throttle or a vacuum leak."),
    rec(DME, 0x04, "Intake VANOS adaptation", Adaptation, num(2, true, 0.1, "°CRK", 1),
        "Learned offset of the intake camshaft timing. Growing offsets suggest a sticking VANOS solenoid or a stretching timing chain."),
    rec(DME, 0x05, "Throttle valve adaptation", Adaptation, num(2, false, 0.01, "%", 2),
        "Learned closed position of the throttle. Carbon build-up moves it over time; a jump after cleaning is normal."),
    rec(DME, 0x10, "Top speed limiter", Coding, num(1, false, 1.0, "km/h", 0),
        "Maximum speed the engine allows. Set by tyre speed rating; raising it beyond the tyres' rating is unsafe."),
    rec(DME, 0x11, "Fuel grade", Coding, Format::Choice { options: &[(0x00, "Premium 95 RON / 91 AKI"), (0x01, "Regular 91 RON / 87 AKI")] },
        "Which fuel the ignition map expects. Coding regular when you run premium costs a little power; the opposite risks knock."),
    // EGS: transmission
    rec(EGS, 0x30, "Clutch fill pressure, A", Adaptation, num(2, true, 0.001, "bar", 3),
        "Learned extra pressure needed to engage clutch A. Rising values mean the transmission is compensating for wear or old fluid."),
    rec(EGS, 0x31, "Clutch fill pressure, B", Adaptation, num(2, true, 0.001, "bar", 3),
        "Learned extra pressure for clutch B. Compare with A: one clutch drifting alone points to that clutch pack."),
    rec(EGS, 0x32, "Shift adaptations learned", Adaptation, num(2, false, 1.0, "shifts", 0),
        "How many shifts the transmission has learned from since the last reset. A drop means someone reset the adaptations."),
    // FRM: lights
    rec(FRM, 0x20, "Daytime running lights", Coding, Format::Choice { options: OFF_ON },
        "Whether the angel eyes stay on as daytime running lights whenever the engine runs."),
    rec(FRM, 0x21, "Comfort turn signal", Coding, num(1, false, 1.0, "flashes", 0),
        "How many times the indicator flashes after a light tap of the stalk."),
    rec(FRM, 0x22, "Fog lights with high beam", Coding, Format::Choice { options: &[(0x00, "Fogs switch off"), (0x01, "Fogs stay on")] },
        "Whether the front fog lights stay on when high beam is switched on. Some countries require them to switch off."),
    // CAS: access
    rec(CAS, 0x40, "Comfort open with remote", Coding, Format::Choice { options: OFF_ON },
        "Holding unlock on the key opens all windows and the sunroof."),
    rec(CAS, 0x41, "Seat belt reminder chime", Coding, Format::Choice { options: OFF_ON },
        "Whether the car chimes when the driver's belt is not fastened."),
    // KOMBI: instrument cluster
    rec(KOMBI, 0x50, "Digital speed display", Coding, Format::Choice { options: OFF_ON },
        "Shows the speed as a number in the on-board computer."),
    rec(KOMBI, 0x51, "Units", Coding, Format::Choice { options: &[(0x00, "Miles, °F"), (0x01, "Kilometres, °C")] },
        "Units used by the cluster and on-board computer."),
];

/// Records defined for one module.
pub fn records_for(address: u8) -> impl Iterator<Item = &'static RecordDef> {
    RECORDS.iter().filter(move |r| r.module == address)
}

/// The BMW X5 (E70) vehicle pack.
pub const PACK: Pack = Pack {
    id: "bmw_e70",
    make: Make::Bmw,
    brand: "BMW",
    model: "X5",
    chassis: "E70",
    years: "2007-2013",
    body: BodyStyle::Suv,
    layout: Layout::Documented,
    dealer_tool: "ISTA",
    modules: MODULES,
    records: RECORDS,
    jobs: JOBS,
    functions: &[
        "clear_codes",
        "coolant_bleed",
        "battery_registration",
        "dme_adaptation_reset",
        "egs_adaptation_reset",
        "actuator_test",
        "coding",
        "key_programming",
        "flash",
    ],
    notes: &[
        "Modules speak KWP2000 over D-CAN through the OBD port. The adapter must support extended addressing (AT CEA).",
        "Key programming uses the car's own CAS security data, read with legitimate access. Keys for a car you cannot already unlock are out of scope.",
    ],
};

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
