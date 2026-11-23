//! Mercedes-Benz GLE (W167, 2019 on). UDS over CAN. Demo layout.

use super::{job, num, rec, uds, OFF_ON};
use crate::snapshot::{Format, RecordDef, RecordKind::*};
use crate::vehicle::{BodyStyle, Job, Layout, Make, Module, Pack, System::*};

pub const ME: Module = uds("ME", "Engine control", "Runs the engine: injection, ignition, camshaft timing, turbo boost and the emissions monitors.", 0x7E0, 0x7E8, Powertrain, (168, 112, "Engine bay, passenger side, on the air box"));
pub const VGS: Module = uds("VGS", "Transmission (9G-TRONIC)", "Controls the 9-speed automatic: shift timing, torque converter lock-up and clutch fill pressures. Sits inside the gearbox.", 0x7E1, 0x7E9, Powertrain, (120, 250, "Inside the transmission, under the centre tunnel"));
pub const ESP: Module = uds(
    "ESP",
    "Brakes / stability control",
    "Anti-lock brakes, stability control, brake hold and the wheel speed sensors.",
    0x740,
    0x748,
    Chassis,
    (72, 80, "Engine bay, driver side, on the brake unit"),
);
pub const EPS: Module = uds(
    "EPS",
    "Electric power steering",
    "Steering assist and steering angle. Needs a steering angle calibration after alignment.",
    0x744,
    0x74C,
    Chassis,
    (96, 140, "On the steering rack"),
);
pub const AIR: Module = uds(
    "AIRMATIC",
    "Air suspension",
    "Ride height and damping. Level calibration is needed after suspension work.",
    0x746,
    0x74E,
    Chassis,
    (120, 360, "Behind the rear seats, left side"),
);
pub const ORC: Module = uds(
    "ORC",
    "Airbag / restraints",
    "Airbags, belt tensioners and crash detection. Read and clear only.",
    0x750,
    0x758,
    Safety,
    (120, 270, "Under the centre console"),
);
pub const EZS: Module = uds(
    "EZS",
    "Ignition and keyless start",
    "Keys, immobilizer and start authorisation. Keys are ordered from Mercedes-Benz by VIN.",
    0x752,
    0x75A,
    Body,
    (104, 196, "Behind the dash, right of the steering column"),
);
pub const IC: Module = uds(
    "IC",
    "Instrument cluster",
    "Digital instruments, warning messages and the service interval (ASSYST PLUS).",
    0x754,
    0x75C,
    Body,
    (88, 176, "Behind the driver display"),
);
pub const HU: Module = uds(
    "HU",
    "MBUX head unit",
    "Screens, navigation, voice control and vehicle settings menus.",
    0x756,
    0x75E,
    Infotainment,
    (150, 178, "Behind the centre screen"),
);
pub const SAM_F: Module = uds(
    "SAM-F",
    "Front signal acquisition module",
    "Front lights, horn, wipers and front fuses. Bulb and LED faults come from here.",
    0x760,
    0x768,
    Body,
    (60, 202, "Driver footwell fuse box"),
);
pub const SAM_R: Module = uds(
    "SAM-R",
    "Rear signal acquisition module",
    "Tail lights, tailgate, trailer socket and rear fuses.",
    0x762,
    0x76A,
    Body,
    (176, 410, "Boot, right side panel"),
);
pub const KLA: Module = uds(
    "KLA",
    "Climate control",
    "Heating, air conditioning, blower and flap motors.",
    0x764,
    0x76C,
    Comfort,
    (120, 184, "Centre of the dashboard"),
);
pub const PTS: Module = uds(
    "PTS",
    "Parking assist (PARKTRONIC)",
    "Parking sensors and the parking assist camera link.",
    0x766,
    0x76E,
    Comfort,
    (64, 418, "Rear bumper, left side"),
);

pub const MODULES: &[Module] = &[
    ME, VGS, ESP, EPS, AIR, ORC, EZS, IC, HU, SAM_F, SAM_R, KLA, PTS,
];

pub const RECORDS: &[RecordDef] = &[
    rec(&ME, 0x2A01, "Mixture adaptation, bank 1", Adaptation, num(2, true, 0.01, "%", 2), "Learned fuel correction for cylinders 1-3. A steady climb usually means unmetered air: a cracked intake hose or a leaking crankcase vent."),
    rec(&ME, 0x2A02, "Mixture adaptation, bank 2", Adaptation, num(2, true, 0.01, "%", 2), "Learned fuel correction for cylinders 4-6. Compare with bank 1 to tell a shared cause from a one-bank fault."),
    rec(&ME, 0x2A10, "Idle speed correction", Adaptation, num(2, true, 1.0, "rpm", 0), "How much the engine has learned to raise idle to stay smooth."),
    rec(&VGS, 0x2B01, "Clutch K1 fill pressure", Adaptation, num(2, true, 0.001, "bar", 3), "Learned extra pressure for clutch K1. Rising values point to old fluid or wear."),
    rec(&VGS, 0x2B02, "Shift adaptations learned", Adaptation, num(2, false, 1.0, "shifts", 0), "Shifts learned from since the last reset. A drop means the adaptations were reset."),
    rec(&SAM_F, 0x2C01, "Daytime running lights", Coding, Format::Choice { options: OFF_ON }, "Whether the LED daytime running lights come on with the engine."),
    rec(&SAM_F, 0x2C02, "Cornering light function", Coding, Format::Choice { options: OFF_ON }, "Fog lamps light the inside of a turn at low speed."),
    rec(&EZS, 0x2D01, "Seat belt warning chime", Coding, Format::Choice { options: OFF_ON }, "Chime when the driver's belt is not fastened."),
];

pub const JOBS: &[Job] = &[
    job("dme_adaptation_reset", &ME),
    job("egs_adaptation_reset", &VGS),
];

pub const PACK: Pack = Pack {
    id: "mercedes_w167",
    make: Make::MercedesBenz,
    brand: "Mercedes-Benz",
    model: "GLE",
    chassis: "W167",
    years: "2019 on",
    body: BodyStyle::Suv,
    layout: Layout::Demo,
    dealer_tool: "XENTRY",
    modules: MODULES,
    records: RECORDS,
    jobs: JOBS,
    functions: &["clear_codes", "dme_adaptation_reset", "egs_adaptation_reset", "actuator_test", "coding", "key_programming", "flash"],
    notes: &[
        "Modules speak UDS. Many coding changes on this generation are protected by Mercedes-Benz online access (SCN coding); ApexDiag only writes what is open to the owner.",
        "Keys are ordered from Mercedes-Benz by VIN and arrive pre-coded for the car. Key programming here means teaching the car a key you legitimately own.",
    ],
};
