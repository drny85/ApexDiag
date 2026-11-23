//! Honda Accord (10th generation, 2018 on). UDS over CAN. Demo layout.

use super::{job, num, rec, uds, OFF_ON};
use crate::snapshot::{Format, RecordDef, RecordKind::*};
use crate::vehicle::{BodyStyle, Job, Layout, Make, Module, Pack, System::*};

pub const PGM_FI: Module = uds(
    "PGM-FI",
    "Engine control (PCM)",
    "Runs the engine: injection, ignition, VTC, turbo boost and the emissions monitors.",
    0x7E0,
    0x7E8,
    Powertrain,
    (164, 116, "Engine bay, passenger side, near the battery"),
);
pub const CVT: Module = uds(
    "CVT",
    "Transmission",
    "Controls the CVT: pulley ratio, start clutch and torque converter lock-up.",
    0x7E1,
    0x7E9,
    Powertrain,
    (120, 232, "Under the centre tunnel, at the transmission"),
);
pub const VSA: Module = uds(
    "VSA",
    "Brakes / stability assist",
    "Anti-lock brakes, vehicle stability assist, brake hold and the wheel speed sensors.",
    0x730,
    0x738,
    Chassis,
    (70, 84, "Engine bay, driver side, on the modulator"),
);
pub const EPS: Module = uds(
    "EPS",
    "Electric power steering",
    "Steering assist and the steering angle sensor.",
    0x732,
    0x73A,
    Chassis,
    (92, 196, "Steering column"),
);
pub const SRS: Module = uds(
    "SRS",
    "Airbag",
    "Airbags, belt pretensioners and the passenger seat sensor. Read and clear only.",
    0x734,
    0x73C,
    Safety,
    (120, 262, "Under the centre console"),
);
pub const METER: Module = uds(
    "METER",
    "Gauge control module",
    "Instruments, warning lights and the Maintenance Minder oil life.",
    0x736,
    0x73E,
    Body,
    (88, 180, "Behind the instruments"),
);
pub const MICU: Module = uds(
    "MICU",
    "Body control (multiplex unit)",
    "Locks, lights, wipers and the owner settings in the Customize menu.",
    0x740,
    0x748,
    Body,
    (60, 200, "Driver side under-dash fuse box"),
);
pub const KEYLESS: Module = uds(
    "KEYLESS",
    "Keyless access and immobilizer",
    "Smart key detection, push-button start and the immobilizer.",
    0x742,
    0x74A,
    Body,
    (150, 206, "Behind the glovebox"),
);
pub const HVAC: Module = uds(
    "HVAC",
    "Climate control",
    "Heating, air conditioning, blower and mode doors.",
    0x744,
    0x74C,
    Comfort,
    (120, 186, "Behind the centre dash"),
);
pub const AUDIO: Module = uds(
    "AUDIO",
    "Display audio",
    "Touch screen, audio, phone and vehicle settings.",
    0x746,
    0x74E,
    Infotainment,
    (150, 176, "Behind the centre screen"),
);
pub const CAMERA: Module = uds("SENSING", "Honda Sensing camera", "Collision mitigation, lane keeping and road departure. Needs aiming after a windscreen change.", 0x750, 0x758, Safety, (120, 168, "Top of the windscreen, behind the mirror"));
pub const RADAR: Module = uds(
    "RADAR",
    "Front radar",
    "Adaptive cruise and collision mitigation. Needs aiming after front-end repairs.",
    0x752,
    0x75A,
    Safety,
    (120, 40, "Behind the front emblem"),
);

pub const MODULES: &[Module] = &[
    PGM_FI, CVT, VSA, EPS, SRS, METER, MICU, KEYLESS, HVAC, AUDIO, CAMERA, RADAR,
];

pub const RECORDS: &[RecordDef] = &[
    rec(&PGM_FI, 0x2401, "Long-term fuel trim", Adaptation, num(2, true, 0.01, "%", 2), "Learned fuel correction. A steady climb usually means an intake or PCV leak."),
    rec(&PGM_FI, 0x2402, "Idle learn", Adaptation, Format::Choice { options: &[(0x00, "Not learned"), (0x01, "Learned")] }, "Whether the engine has completed its idle learn. Needed after a battery disconnect or PCM reset."),
    rec(&CVT, 0x2501, "Start clutch learned value", Adaptation, num(2, true, 0.001, "bar", 3), "Learned pressure to engage the start clutch. Rising values point to old CVT fluid."),
    rec(&MICU, 0x2601, "Auto door lock", Coding, Format::Choice { options: &[(0x00, "Off"), (0x01, "At 15 km/h"), (0x02, "When shifted out of P")] }, "When the doors lock themselves."),
    rec(&MICU, 0x2602, "Keyless lock answer back", Coding, Format::Choice { options: OFF_ON }, "Horn chirp and light flash when locking with the key."),
    rec(&MICU, 0x2603, "Interior light dimming time", Coding, num(1, false, 1.0, "s", 0), "How long the interior lights take to fade after the doors close."),
];

pub const JOBS: &[Job] = &[
    job("dme_adaptation_reset", &PGM_FI),
    job("egs_adaptation_reset", &CVT),
];

pub const PACK: Pack = Pack {
    id: "honda_accord",
    make: Make::Honda,
    brand: "Honda",
    model: "Accord",
    chassis: "10th gen",
    years: "2018 on",
    body: BodyStyle::Sedan,
    layout: Layout::Demo,
    dealer_tool: "i-HDS",
    modules: MODULES,
    records: RECORDS,
    jobs: JOBS,
    functions: &["clear_codes", "dme_adaptation_reset", "egs_adaptation_reset", "actuator_test", "coding", "key_programming", "flash"],
    notes: &[
        "Many Honda body modules talk on 29-bit CAN IDs. This demo layout uses 11-bit IDs; the real map comes when it is read from a car.",
        "After a PCM reset or battery disconnect, Honda engines need an idle learn: start the engine and let it idle with all accessories off for about 10 minutes.",
    ],
};
