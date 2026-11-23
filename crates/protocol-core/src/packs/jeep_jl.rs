//! Jeep Wrangler (JL, 2018 on). UDS over CAN. Demo layout.
//!
//! Module names follow the maker's own short names (PCM, ORC, RFH, SCCM).
//! The CAN IDs and record identifiers are not confirmed on a real car yet.

use super::{job, num, rec, uds, OFF_ON};
use crate::snapshot::{Format, RecordDef, RecordKind::*};
use crate::vehicle::{BodyStyle, Job, Layout, Make, Module, Pack, System::*};

pub const PCM: Module = uds(
    "PCM",
    "Engine control",
    "Runs the engine: injection, ignition, cam timing, engine stop-start and the emissions monitors.",
    0x7E0,
    0x7E8,
    Powertrain,
    (120, 96, "Engine bay, driver side, on the inner fender"),
);
pub const TCM: Module = uds(
    "TCM",
    "Transmission",
    "Shift timing, clutch pressures and torque converter lock-up for the 8-speed automatic.",
    0x7E1,
    0x7E9,
    Powertrain,
    (120, 232, "On the transmission, under the centre tunnel"),
);
pub const ABS: Module = uds(
    "ABS",
    "Brakes / stability control",
    "Anti-lock brakes, electronic stability control, hill descent, off-road traction modes and the wheel speed sensors.",
    0x720,
    0x728,
    Chassis,
    (66, 88, "Engine bay, driver side, on the brake modulator"),
);
pub const EPS: Module = uds(
    "EPS",
    "Power steering",
    "Electro-hydraulic steering assist: the pump runs on its own motor, not off the engine belt.",
    0x722,
    0x72A,
    Chassis,
    (172, 70, "Front of the engine bay, passenger side"),
);
pub const SCCM: Module = uds(
    "SCCM",
    "Steering column control",
    "Turn signal and wiper stalks, the clock spring and the steering angle sensor used by stability control.",
    0x724,
    0x72C,
    Chassis,
    (94, 198, "Top of the steering column"),
);
pub const ORC: Module = uds(
    "ORC",
    "Airbag (occupant restraint controller)",
    "Airbags, seat belt pretensioners and the crash sensors. Read and clear only.",
    0x726,
    0x72E,
    Safety,
    (120, 262, "Under the centre console"),
);
pub const IPC: Module = uds(
    "IPC",
    "Instrument cluster",
    "Gauges, warning lights, the cluster display and the oil life reminder.",
    0x740,
    0x748,
    Body,
    (74, 166, "Behind the instruments"),
);
pub const BCM: Module = uds(
    "BCM",
    "Body control",
    "Lights, locks, wipers, the battery monitoring for stop-start and the owner settings.",
    0x742,
    0x74A,
    Body,
    (42, 204, "Driver side, behind the kick panel"),
);
pub const RFH: Module = uds(
    "RFH",
    "Radio frequency hub",
    "Key fob receiver, keyless go, the immobilizer and the tyre pressure sensors.",
    0x744,
    0x74C,
    Body,
    (150, 300, "Under the rear seat area"),
);
pub const HVAC: Module = uds(
    "HVAC",
    "Climate control",
    "Heating, air conditioning, blower and the blend doors.",
    0x746,
    0x74E,
    Comfort,
    (124, 182, "Behind the centre dash"),
);
pub const RADIO: Module = uds(
    "RADIO",
    "Uconnect radio",
    "Touch screen, audio, phone, the off-road pages and the owner settings menu.",
    0x750,
    0x758,
    Infotainment,
    (162, 166, "Behind the centre screen"),
);
pub const PAM: Module = uds(
    "PAM",
    "Park assist",
    "Rear park sensors and their warnings. Fitted to some trims only.",
    0x752,
    0x75A,
    Safety,
    (120, 432, "Rear bumper"),
);
pub const SGW: Module = uds(
    "SGW",
    "Security gateway",
    "Sits between the OBD port and the car's networks. Reading passes through; clearing codes, routines and coding need an unlocked tool.",
    0x754,
    0x75C,
    Body,
    (66, 230, "Under the dash, driver side, above the OBD port"),
);

pub const MODULES: &[Module] = &[
    PCM, TCM, ABS, EPS, SCCM, ORC, IPC, BCM, RFH, HVAC, RADIO, PAM, SGW,
];

pub const RECORDS: &[RecordDef] = &[
    rec(&PCM, 0x2701, "Long-term fuel trim, bank 1", Adaptation, num(2, true, 0.01, "%", 2), "Learned fuel correction for the left bank. A steady climb usually means an intake or crankcase vent leak."),
    rec(&PCM, 0x2702, "Long-term fuel trim, bank 2", Adaptation, num(2, true, 0.01, "%", 2), "Learned fuel correction for the right bank. Compare with bank 1: one bank alone points to that side of the engine."),
    rec(&TCM, 0x2801, "Clutch fill adaptation", Adaptation, num(2, true, 0.001, "bar", 3), "Learned fill pressure for the shifting clutches. Reset it after a valve body or fluid service, then let the transmission relearn."),
    rec(&BCM, 0x2901, "Auto door locks", Coding, Format::Choice { options: OFF_ON }, "Locks the doors once the vehicle moves."),
    rec(&BCM, 0x2902, "Horn with remote lock", Coding, Format::Choice { options: &[(0x00, "Off"), (0x01, "First press"), (0x02, "Second press")] }, "When the horn sounds as you lock with the key fob."),
    rec(&BCM, 0x2903, "Headlight off delay", Coding, num(1, false, 1.0, "s", 0), "How long the headlights stay on after you leave the vehicle."),
];

pub const JOBS: &[Job] = &[
    job("dme_adaptation_reset", &PCM),
    job("egs_adaptation_reset", &TCM),
];

pub const PACK: Pack = Pack {
    id: "jeep_jl",
    make: Make::Jeep,
    brand: "Jeep",
    model: "Wrangler",
    chassis: "JL",
    years: "2018 on",
    body: BodyStyle::Offroad,
    layout: Layout::Demo,
    dealer_tool: "wiTECH",
    modules: MODULES,
    records: RECORDS,
    jobs: JOBS,
    functions: &["clear_codes", "dme_adaptation_reset", "egs_adaptation_reset", "actuator_test", "coding", "key_programming", "flash"],
    notes: &[
        "From 2018 these vehicles have a security gateway behind the OBD port. Reading codes and live data works with any tool. Clearing codes, running routines, coding and key programming are blocked until the tool is unlocked with a registered account.",
        "Engine stop-start depends on a healthy battery. A weak battery or auxiliary battery is the usual reason stop-start stops working, often before any warning light.",
        "Bigger tyres or a changed axle ratio need the tyre size set in the vehicle configuration, or the speedometer, shift points and stability control will be wrong.",
        "This demo layout uses 11-bit IDs; the real module map comes when it is read from a car.",
    ],
};
