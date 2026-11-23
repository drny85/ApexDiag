//! Toyota Camry (XV70, 2018 on). UDS over CAN. Demo layout.

use super::{job, num, rec, uds, OFF_ON};
use crate::snapshot::{Format, RecordDef, RecordKind::*};
use crate::vehicle::{BodyStyle, Job, Layout, Make, Module, Pack, System::*};

pub const ECM: Module = uds("ECM", "Engine control", "Runs the engine and the 8-speed transmission strategy: injection, ignition, VVT-i, throttle and the emissions monitors.", 0x7E0, 0x7E8, Powertrain, (166, 118, "Engine bay, passenger side, by the battery"));
pub const ECT: Module = uds(
    "ECT",
    "Transmission",
    "Shift timing and torque converter lock-up for the Direct Shift 8-speed automatic.",
    0x7E1,
    0x7E9,
    Powertrain,
    (120, 230, "Under the centre tunnel, at the transmission"),
);
pub const ABS: Module = uds(
    "ABS/VSC",
    "Brakes / stability control",
    "Anti-lock brakes, vehicle stability control and the wheel speed sensors.",
    0x7B0,
    0x7B8,
    Chassis,
    (70, 84, "Engine bay, driver side, on the brake actuator"),
);
pub const EPS: Module = uds(
    "EPS",
    "Electric power steering",
    "Steering assist and the steering angle sensor.",
    0x7A1,
    0x7A9,
    Chassis,
    (92, 196, "Steering column"),
);
pub const SRS: Module = uds(
    "SRS",
    "Airbag",
    "Airbags, belt pretensioners and the occupant sensor. Read and clear only.",
    0x780,
    0x788,
    Safety,
    (120, 262, "Under the centre console"),
);
pub const METER: Module = uds(
    "METER",
    "Combination meter",
    "Gauges, warning lights, multi-information display and the oil maintenance reminder.",
    0x7C0,
    0x7C8,
    Body,
    (88, 180, "Behind the instruments"),
);
pub const AC: Module = uds(
    "A/C",
    "Air conditioner amplifier",
    "Climate control: blower, damper motors, compressor request and sensors.",
    0x7C4,
    0x7CC,
    Comfort,
    (120, 186, "Behind the centre dash"),
);
pub const BODY: Module = uds(
    "MAIN BODY",
    "Main body ECU",
    "Door locks, interior lights, wipers and many owner customizations.",
    0x7D0,
    0x7D8,
    Body,
    (60, 200, "Driver side instrument panel, by the fuse block"),
);
pub const SMART: Module = uds(
    "SMART KEY",
    "Certification ECU (smart key)",
    "Smart key detection, push-button start and the immobilizer.",
    0x7D2,
    0x7DA,
    Body,
    (150, 206, "Behind the glovebox"),
);
pub const PCS: Module = uds("TSS-P", "Toyota Safety Sense camera", "Pre-collision, lane departure and automatic high beam. Needs calibration after a windscreen change.", 0x750, 0x758, Safety, (120, 170, "Top of the windscreen, behind the mirror"));

pub const MODULES: &[Module] = &[ECM, ECT, ABS, EPS, SRS, METER, AC, BODY, SMART, PCS];

pub const RECORDS: &[RecordDef] = &[
    rec(&ECM, 0x2101, "Long-term fuel trim, bank 1", Adaptation, num(2, true, 0.01, "%", 2), "Learned fuel correction. Near 0 is healthy; a steady climb usually means an intake leak or a dirty air flow sensor."),
    rec(&ECM, 0x2102, "Idle air learning", Adaptation, num(2, true, 0.1, "%", 1), "Learned throttle opening at idle. Grows as the throttle body gets dirty."),
    rec(&BODY, 0x2201, "Auto door lock", Coding, Format::Choice { options: &[(0x00, "Off"), (0x01, "When driving off (speed-linked)"), (0x02, "When shifting out of P")] }, "When the doors lock themselves."),
    rec(&BODY, 0x2202, "Auto door unlock", Coding, Format::Choice { options: &[(0x00, "Off"), (0x01, "When shifting into P"), (0x02, "When the driver's door opens")] }, "When the doors unlock themselves."),
    rec(&BODY, 0x2203, "Interior light fade-out", Coding, num(1, false, 1.0, "s", 0), "How long the interior lights stay on after the doors close."),
    rec(&METER, 0x2301, "Seat belt reminder", Coding, Format::Choice { options: OFF_ON }, "Chime when a front belt is not fastened."),
];

pub const JOBS: &[Job] = &[job("dme_adaptation_reset", &ECM)];

pub const PACK: Pack = Pack {
    id: "toyota_xv70",
    make: Make::Toyota,
    brand: "Toyota",
    model: "Camry",
    chassis: "XV70",
    years: "2018 on",
    body: BodyStyle::Sedan,
    layout: Layout::Demo,
    dealer_tool: "Techstream",
    modules: MODULES,
    records: RECORDS,
    jobs: JOBS,
    functions: &["clear_codes", "dme_adaptation_reset", "actuator_test", "coding", "key_programming", "flash"],
    notes: &[
        "Toyota calls owner settings \"customize\" and actuator tests \"active tests\". ApexDiag uses the same ideas with plain names.",
        "Smart key registration needs the car's own security access and at least one working key.",
    ],
};
