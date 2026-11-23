//! A simulated ELM327 plugged into a simulated car.
//!
//! It answers the same text protocol a real adapter does, so the whole stack
//! (client, parser, ISO-TP, UI) can be developed and tested without a car.
//! Values move over time so gauges have something to show.
//!
//! Five demo cars are available, one per vehicle pack: a BMW X5 (KWP2000
//! with BMW extended addressing), a Mercedes-Benz GLE, a Toyota Camry, a
//! Honda Accord and a Jeep Wrangler (UDS on 11-bit CAN). Everything they report is invented, and
//! every demo VIN starts with `SIMULATED`.

use crate::{HardwareTransport, Result, TransportError};
use async_trait::async_trait;
use protocol_core::isotp::{segment, Addressing, FlowControl, Segmented};
use protocol_core::packs::{honda_accord, jeep_jl, mercedes_w167, toyota_xv70};
use protocol_core::snapshot::RecordKind;
use protocol_core::vehicle::{Link, Module, Pack, BMW_TESTER_ADDRESS, BMW_TESTER_ID};
use protocol_core::{bmw_e70, CanId};
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

/// One of the simulator's demo cars.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct SimCar {
    pub id: &'static str,
    pub label: &'static str,
    /// What is wrong with it, so the demo has a story.
    pub story: &'static str,
    pub vin: &'static str,
    #[serde(skip)]
    pub pack: &'static Pack,
}

pub const SIM_CARS: [SimCar; 5] = [
    SimCar {
        id: "bmw_x5",
        label: "BMW X5 (E70)",
        story: "Lean on both banks from a small vacuum leak, an oil level sensor fault and a wheel speed sensor that drops out.",
        vin: "SIMULATED0APEX001",
        pack: &bmw_e70::PACK,
    },
    SimCar {
        id: "mercedes_gle",
        label: "Mercedes-Benz GLE (W167)",
        story: "Engine light off and emissions ready, but the air suspension logs a front left pressure loss.",
        vin: "SIMULATED0GLE0002",
        pack: &mercedes_w167::PACK,
    },
    SimCar {
        id: "toyota_camry",
        label: "Toyota Camry (XV70)",
        story: "No stored codes, but three emissions monitors are incomplete: the codes were cleared recently.",
        vin: "SIMULATED0CAM0003",
        pack: &toyota_xv70::PACK,
    },
    SimCar {
        id: "honda_accord",
        label: "Honda Accord (10th gen)",
        story: "Engine light on for catalyst efficiency, with a freeze frame from the moment it failed.",
        vin: "SIMULATED0ACC0004",
        pack: &honda_accord::PACK,
    },
    SimCar {
        id: "jeep_wrangler",
        label: "Jeep Wrangler (JL)",
        story: "Engine light on for a small EVAP leak, and stop-start has quit: the charging voltage runs low with the engine on.",
        vin: "SIMULATED0JLW0005",
        pack: &jeep_jl::PACK,
    },
];

/// The default demo car's VIN (the BMW).
pub const SIM_VIN: &str = SIM_CARS[0].vin;

pub fn sim_car(id: &str) -> Option<&'static SimCar> {
    SIM_CARS.iter().find(|c| c.id == id)
}

pub fn sim_car_by_vin(vin: &str) -> Option<&'static SimCar> {
    SIM_CARS.iter().find(|c| c.vin == vin)
}

/// Descriptions for the simulator's invented module fault entries. These
/// are demo text, not real manufacturer fault meanings.
pub fn sim_fault_text(pack: &str, address: u8, code: &str) -> Option<&'static str> {
    Some(match (pack, address, code) {
        ("bmw_e70", 0x12, "29F3") => "Demo: oil level sensor signal implausible",
        ("bmw_e70", 0x12, "2A82") => "Demo: intake VANOS solenoid, response too slow",
        ("bmw_e70", 0x29, "5E20") => "Demo: wheel speed sensor rear left, signal intermittent",
        ("mercedes_w167", 0x46, "C1525-64") => {
            "Demo: front left air spring loses pressure while parked"
        }
        ("mercedes_w167", 0xE0, "P0016-00") => {
            "Demo: crankshaft / camshaft position correlation, bank 1 (history)"
        }
        ("mercedes_w167", 0x56, "U0155-87") => {
            "Demo: lost communication with the instrument cluster (history)"
        }
        ("toyota_xv70", 0xB0, "C1241-16") => "Demo: low battery voltage seen during a crank",
        ("toyota_xv70", 0xD2, "B2785-00") => "Demo: smart key, ignition hold monitor (history)",
        ("toyota_xv70", 0xE0, "P0171-00") => "Demo: system too lean, bank 1 (pending)",
        ("honda_accord", 0xE0, "P0420-00") => "Demo: catalyst efficiency below threshold, bank 1",
        ("honda_accord", 0x50, "U0301-00") => {
            "Demo: camera software or aim mismatch after a windscreen change"
        }
        ("jeep_jl", 0xE0, "P0456-00") => "Demo: EVAP system, very small leak detected",
        ("jeep_jl", 0xE0, "P0562-00") => "Demo: system voltage low (pending)",
        ("jeep_jl", 0x42, "U3003-16") => {
            "Demo: battery voltage below threshold, stop-start disabled"
        }
        ("jeep_jl", 0x24, "C0460-00") => {
            "Demo: steering angle sensor not calibrated after an alignment (history)"
        }
        _ => return None,
    })
}

/// How the simulated car is sitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimEngine {
    /// Ignition on, engine off: what most service functions need.
    IgnitionOn,
    /// Engine idling in Park.
    Idling,
}

/// Adapter settings, reset by `AT Z`.
#[derive(Debug, Clone, Copy)]
struct Adapter {
    echo: bool,
    headers: bool,
    header_id: CanId,
    cea: Option<u8>,
}

impl Default for Adapter {
    fn default() -> Self {
        Self {
            echo: true,
            headers: false,
            header_id: 0x7DF,
            cea: None,
        }
    }
}

/// Where the engine keeps its learned fuel trims.
#[derive(Debug, Clone, Copy)]
struct Trims {
    module: u8,
    bank1: u16,
    bank2: Option<u16>,
}

/// The code that stored a freeze frame and the PID values it captured.
type Freeze = ([u8; 2], BTreeMap<u8, Vec<u8>>);

pub struct ElmSimulator {
    connected: bool,
    out: VecDeque<u8>,
    start: Instant,
    engine: SimEngine,
    charger: bool,
    adapter: Adapter,
    car: &'static SimCar,
    stored: Vec<[u8; 2]>,
    pending: Vec<[u8; 2]>,
    permanent: Vec<[u8; 2]>,
    /// Readiness monitors (PID 01 bytes C and D): supported, incomplete.
    monitors: (u8, u8),
    /// Freeze frame 0: the code that set it and the captured PID values.
    freeze: Option<Freeze>,
    /// Module address -> fault memory entries (KWP `[hi lo status]`, UDS
    /// `[hi mid lo status]`). Modules missing from the map are not fitted.
    faults: BTreeMap<u8, Vec<Vec<u8>>>,
    /// (module address, record id) -> raw value, for the Time Machine.
    records: BTreeMap<(u8, u16), Vec<u8>>,
    /// The underlying fuel error per bank, in percent. Learned trims sit
    /// near it; short-term trims cover whatever is not learned.
    leak: (f64, f64),
    trims: Trims,
    /// The catalytic converter no longer stores oxygen (the Accord's story).
    cat_worn: bool,
    /// The alternator undercharges with the engine running (the Jeep's story).
    low_charge: bool,
    /// Demo only: when a one-off stumble started (seconds since start).
    stumble_at: Option<f64>,
    /// Answers several PIDs per request (most CAN cars do). Tests turn it
    /// off to act like a car that only takes one at a time.
    pub multi_pid: bool,
    /// Requests seen, for tests.
    pub log: Vec<String>,
}

impl Default for ElmSimulator {
    fn default() -> Self {
        Self::new(SimEngine::Idling)
    }
}

impl ElmSimulator {
    /// The default demo car (BMW X5).
    pub fn new(engine: SimEngine) -> Self {
        Self::with_car(&SIM_CARS[0], engine)
    }

    pub fn with_car(car: &'static SimCar, engine: SimEngine) -> Self {
        let mut sim = Self {
            connected: false,
            out: VecDeque::new(),
            start: Instant::now(),
            engine,
            charger: false,
            adapter: Adapter::default(),
            car,
            stored: vec![],
            pending: vec![],
            permanent: vec![],
            monitors: (0x65, 0x00),
            freeze: None,
            faults: BTreeMap::new(),
            records: BTreeMap::new(),
            leak: (0.0, 0.0),
            cat_worn: false,
            low_charge: false,
            stumble_at: None,
            multi_pid: true,
            trims: Trims {
                module: 0,
                bank1: 0,
                bank2: None,
            },
            log: Vec::new(),
        };
        sim.load_story();
        sim
    }

    pub fn car(&self) -> &'static SimCar {
        self.car
    }

    /// Fills in the demo car's faults, records and live-data quirks.
    fn load_story(&mut self) {
        let w = |v: i16| v.to_be_bytes().to_vec();
        let kwp = |hi: u8, lo: u8, st: u8| vec![hi, lo, st];
        let uds = |code: &str, ft: u8, st: u8| {
            let [hi, lo] = sae(code);
            vec![hi, lo, ft, st]
        };
        let fitted = |pack: &Pack, absent: &[u8]| {
            pack.modules
                .iter()
                .filter(|m| !absent.contains(&m.address))
                .map(|m| (m.address, Vec::new()))
                .collect::<BTreeMap<u8, Vec<Vec<u8>>>>()
        };
        match self.car.id {
            "mercedes_gle" => {
                use mercedes_w167::*;
                self.faults = fitted(&PACK, &[PTS.address]);
                self.faults
                    .insert(AIR.address, vec![uds("C1525", 0x64, 0x09)]);
                self.faults
                    .insert(ME.address, vec![uds("P0016", 0x00, 0x08)]);
                self.faults
                    .insert(HU.address, vec![uds("U0155", 0x87, 0x08)]);
                self.records = BTreeMap::from([
                    ((ME.address, 0x2A01), w(310)),
                    ((ME.address, 0x2A02), w(280)),
                    ((ME.address, 0x2A10), w(12)),
                    ((VGS.address, 0x2B01), w(62)),
                    ((VGS.address, 0x2B02), w(2210)),
                    ((SAM_F.address, 0x2C01), vec![1]),
                    ((SAM_F.address, 0x2C02), vec![1]),
                    ((EZS.address, 0x2D01), vec![1]),
                ]);
                self.monitors = (0x65, 0x00);
                self.leak = (3.1, 2.8);
                self.trims = Trims {
                    module: ME.address,
                    bank1: 0x2A01,
                    bank2: Some(0x2A02),
                };
            }
            "toyota_camry" => {
                use toyota_xv70::*;
                self.faults = fitted(&PACK, &[]);
                self.faults
                    .insert(ABS.address, vec![uds("C1241", 0x16, 0x08)]);
                self.faults
                    .insert(SMART.address, vec![uds("B2785", 0x00, 0x08)]);
                self.faults
                    .insert(ECM.address, vec![uds("P0171", 0x00, 0x24)]);
                self.pending = vec![sae("P0171")];
                self.records = BTreeMap::from([
                    ((ECM.address, 0x2101), w(420)),
                    ((ECM.address, 0x2102), w(38)),
                    ((BODY.address, 0x2201), vec![1]),
                    ((BODY.address, 0x2202), vec![1]),
                    ((BODY.address, 0x2203), vec![15]),
                    ((METER.address, 0x2301), vec![1]),
                ]);
                // Catalyst, EVAP and O2 sensor monitors have not run yet.
                self.monitors = (0x65, 0x25);
                self.leak = (6.8, 0.0);
                self.trims = Trims {
                    module: ECM.address,
                    bank1: 0x2101,
                    bank2: None,
                };
            }
            "honda_accord" => {
                use honda_accord::*;
                self.faults = fitted(&PACK, &[]);
                self.faults
                    .insert(PGM_FI.address, vec![uds("P0420", 0x00, 0x2F)]);
                self.faults
                    .insert(CAMERA.address, vec![uds("U0301", 0x00, 0x08)]);
                self.stored = vec![sae("P0420")];
                self.permanent = vec![sae("P0420")];
                self.records = BTreeMap::from([
                    ((PGM_FI.address, 0x2401), w(160)),
                    ((PGM_FI.address, 0x2402), vec![1]),
                    ((CVT.address, 0x2501), w(45)),
                    ((MICU.address, 0x2601), vec![1]),
                    ((MICU.address, 0x2602), vec![0]),
                    ((MICU.address, 0x2603), vec![15]),
                ]);
                self.monitors = (0x65, 0x00);
                self.freeze = Some((
                    sae("P0420"),
                    BTreeMap::from([
                        (0x04, vec![97]),         // load 38 %
                        (0x05, vec![131]),        // coolant 91 °C
                        (0x06, vec![131]),        // STFT +2.3 %
                        (0x07, vec![130]),        // LTFT +1.6 %
                        (0x0B, vec![52]),         // MAP 52 kPa
                        (0x0C, vec![0x23, 0x00]), // 2240 rpm
                        (0x0D, vec![72]),         // 72 km/h
                        (0x0F, vec![64]),         // intake 24 °C
                        (0x11, vec![46]),         // throttle 18 %
                    ]),
                ));
                self.leak = (1.6, 0.0);
                self.cat_worn = true;
                self.trims = Trims {
                    module: PGM_FI.address,
                    bank1: 0x2401,
                    bank2: None,
                };
            }
            "jeep_wrangler" => {
                use jeep_jl::*;
                // Park assist is not fitted on this trim.
                self.faults = fitted(&PACK, &[PAM.address]);
                self.faults.insert(
                    PCM.address,
                    vec![uds("P0456", 0x00, 0x2F), uds("P0562", 0x00, 0x24)],
                );
                self.faults
                    .insert(BCM.address, vec![uds("U3003", 0x16, 0x09)]);
                self.faults
                    .insert(SCCM.address, vec![uds("C0460", 0x00, 0x08)]);
                self.stored = vec![sae("P0456")];
                self.pending = vec![sae("P0562")];
                self.records = BTreeMap::from([
                    ((PCM.address, 0x2701), w(120)),
                    ((PCM.address, 0x2702), w(95)),
                    ((TCM.address, 0x2801), w(52)),
                    ((BCM.address, 0x2901), vec![1]),
                    ((BCM.address, 0x2902), vec![1]),
                    ((BCM.address, 0x2903), vec![30]),
                ]);
                self.monitors = (0x65, 0x00);
                self.freeze = Some((
                    sae("P0456"),
                    BTreeMap::from([
                        (0x04, vec![46]),         // load 18 %
                        (0x05, vec![120]),        // coolant 80 °C
                        (0x06, vec![129]),        // STFT +0.8 %
                        (0x07, vec![129]),        // LTFT +0.8 %
                        (0x0B, vec![38]),         // MAP 38 kPa
                        (0x0C, vec![0x0A, 0xF0]), // 700 rpm
                        (0x0D, vec![0]),          // parked
                        (0x0F, vec![52]),         // intake 12 °C
                        (0x11, vec![38]),         // throttle 15 %
                    ]),
                ));
                self.leak = (1.2, 1.0);
                self.low_charge = true;
                self.trims = Trims {
                    module: PCM.address,
                    bank1: 0x2701,
                    bank2: Some(0x2702),
                };
            }
            _ => {
                use bmw_e70::*;
                self.faults = fitted(&PACK, &[PDC.address, FZD.address]);
                self.faults.insert(
                    DME.address,
                    vec![kwp(0x29, 0xF3, 0x24), kwp(0x2A, 0x82, 0x20)],
                );
                self.faults.insert(DSC.address, vec![kwp(0x5E, 0x20, 0x24)]);
                self.stored = vec![sae("P0171"), sae("P0174")];
                self.pending = vec![sae("P0300")];
                self.records = BTreeMap::from([
                    ((DME.address, 0x01), w(940)),
                    ((DME.address, 0x02), w(860)),
                    ((DME.address, 0x03), w(18)),
                    ((DME.address, 0x04), w(-24)),
                    ((DME.address, 0x05), w(62)),
                    ((DME.address, 0x10), vec![210]),
                    ((DME.address, 0x11), vec![0x00]),
                    ((EGS.address, 0x30), w(84)),
                    ((EGS.address, 0x31), w(41)),
                    ((EGS.address, 0x32), w(1843)),
                    ((FRM.address, 0x20), vec![0x01]),
                    ((FRM.address, 0x21), vec![5]),
                    ((FRM.address, 0x22), vec![0x00]),
                    ((CAS.address, 0x40), vec![0x01]),
                    ((CAS.address, 0x41), vec![0x01]),
                    ((KOMBI.address, 0x50), vec![0x00]),
                    ((KOMBI.address, 0x51), vec![0x00]),
                ]);
                // EVAP has not finished since the last clear.
                self.monitors = (0x65, 0x04);
                self.freeze = Some((
                    sae("P0171"),
                    BTreeMap::from([
                        (0x04, vec![61]),         // load 24 %
                        (0x05, vec![128]),        // coolant 88 °C
                        (0x06, vec![136]),        // STFT +6.2 %
                        (0x07, vec![140]),        // LTFT +9.4 %
                        (0x0B, vec![33]),         // MAP 33 kPa
                        (0x0C, vec![0x0B, 0x40]), // 720 rpm
                        (0x0D, vec![0]),          // stationary
                        (0x0F, vec![71]),         // intake 31 °C
                        (0x11, vec![21]),         // throttle 8 %
                    ]),
                ));
                self.leak = (9.4, 8.6);
                self.trims = Trims {
                    module: DME.address,
                    bank1: 0x01,
                    bank2: Some(0x02),
                };
            }
        }
    }

    /// Demo only: pretend the repair for `code` was done, so the Repair
    /// Copilot's re-test has something to show. Returns false when the demo
    /// car has nothing to repair for that code.
    pub fn apply_demo_repair(&mut self, code: &str) -> bool {
        match code.get(..5) {
            Some("P0171" | "P0174" | "P0172" | "P0175" | "P0101") if self.leak.0.abs() > 2.0 => {
                self.leak = (0.4, if self.two_banks() { 0.3 } else { 0.0 });
                // The engine relearns its trims over the next drive.
                let t = self.trims;
                for (id, v) in [(Some(t.bank1), 0.4), (t.bank2, 0.3)] {
                    if let Some(id) = id {
                        let raw = ((v * 100.0) as i16).to_be_bytes().to_vec();
                        self.records.insert((t.module, id), raw);
                    }
                }
                true
            }
            Some("P0420" | "P0430") if self.cat_worn => {
                self.cat_worn = false;
                true
            }
            Some("P0562" | "U3003") if self.low_charge => {
                self.low_charge = false;
                true
            }
            _ => false,
        }
    }

    /// Every record's current raw value.
    pub fn records(&self) -> Vec<((u8, u16), Vec<u8>)> {
        self.records.iter().map(|(k, v)| (*k, v.clone())).collect()
    }

    /// A record's current raw value.
    pub fn record(&self, address: u8, id: u16) -> Option<&[u8]> {
        self.records.get(&(address, id)).map(Vec::as_slice)
    }

    /// What a successful adaptation reset does: the learned values go back
    /// to zero and the module starts relearning.
    pub fn reset_adaptations(&mut self, address: u8) {
        for def in self.car.pack.records_for(address) {
            if def.kind == RecordKind::Adaptation {
                if let Some(v) = self.records.get_mut(&(address, def.id)) {
                    v.iter_mut().for_each(|b| *b = 0);
                }
            }
        }
    }

    /// Learned long-term fuel trim for a bank, in percent.
    fn ltft(&self, bank: u8) -> f64 {
        let id = if bank == 1 {
            Some(self.trims.bank1)
        } else {
            self.trims.bank2
        };
        id.and_then(|id| {
            let raw = self.record(self.trims.module, id)?;
            self.car
                .pack
                .records_for(self.trims.module)
                .find(|d| d.id == id)?
                .number(raw)
        })
        .unwrap_or(0.0)
    }

    pub fn set_engine(&mut self, engine: SimEngine) {
        self.engine = engine;
    }

    /// Demo only: makes the idling engine stumble once, for about a second,
    /// like an intermittent misfire. Instant Replay is demonstrated on it.
    pub fn stumble(&mut self) {
        self.stumble_at = Some(self.t());
    }

    /// How deep into the stumble the engine is right now, 0.0 to 1.0.
    fn stumble_depth(&self) -> f64 {
        const LENGTH: f64 = 1.2;
        match self.stumble_at {
            Some(at) if self.running() => {
                let x = (self.t() - at) / LENGTH;
                if (0.0..1.0).contains(&x) {
                    (x * std::f64::consts::PI).sin()
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    pub fn engine(&self) -> SimEngine {
        self.engine
    }

    /// Simulates a battery charger on the car (raises resting voltage).
    pub fn set_charger(&mut self, on: bool) {
        self.charger = on;
    }

    /// System voltage the simulated car reports.
    fn volts(&self) -> f64 {
        if self.running() {
            if self.low_charge {
                12.9
            } else {
                14.2
            }
        } else if self.charger {
            13.6
        } else {
            12.5
        }
    }

    fn t(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn running(&self) -> bool {
        self.engine == SimEngine::Idling
    }

    fn two_banks(&self) -> bool {
        self.trims.bank2.is_some()
    }

    fn handle(&mut self, line: &str) -> String {
        let cmd = line.trim().to_ascii_uppercase();
        if self.log.len() >= 500 {
            self.log.drain(..250);
        }
        self.log.push(cmd.clone());
        let body = if let Some(at) = cmd.strip_prefix("AT") {
            self.handle_at(at.trim())
        } else {
            self.handle_request(&cmd)
        };
        let mut out = String::new();
        if self.adapter.echo {
            out.push_str(line.trim());
            out.push('\r');
        }
        out.push_str(&body);
        out.push_str("\r\r>");
        out
    }

    fn handle_at(&mut self, at: &str) -> String {
        let compact: String = at.chars().filter(|c| !c.is_whitespace()).collect();
        let a = &mut self.adapter;
        match compact.as_str() {
            "Z" => {
                *a = Adapter::default();
                self.connected = true;
                "\rELM327 v1.5".into()
            }
            "I" => "ELM327 v1.5".into(),
            "E0" => {
                a.echo = false;
                "OK".into()
            }
            "E1" => {
                a.echo = true;
                "OK".into()
            }
            "H1" => {
                a.headers = true;
                "OK".into()
            }
            "H0" => {
                a.headers = false;
                "OK".into()
            }
            "RV" => {
                let v = self.volts() + (self.t() * 0.7).sin() * 0.05;
                format!("{v:.1}V")
            }
            "DP" => "ISO 15765-4 (CAN 11/500)".into(),
            "CEA" => {
                a.cea = None;
                "OK".into()
            }
            c if c.starts_with("CEA") => match u8::from_str_radix(&c[3..], 16) {
                Ok(x) => {
                    a.cea = Some(x);
                    "OK".into()
                }
                Err(_) => "?".into(),
            },
            c if c.starts_with("SH") => match CanId::from_str_radix(&c[2..], 16) {
                Ok(id) => {
                    a.header_id = id;
                    "OK".into()
                }
                Err(_) => "?".into(),
            },
            // Accepted and ignored: formatting, timing, filters, flow control.
            c if [
                "L0", "L1", "S0", "S1", "AT0", "AT1", "AT2", "SP6", "CAF0", "CAF1", "CRA",
            ]
            .contains(&c)
                || c.starts_with("CRA")
                || c.starts_with("FC")
                || c.starts_with("ST") =>
            {
                "OK".into()
            }
            _ => "?".into(),
        }
    }

    fn handle_request(&mut self, cmd: &str) -> String {
        let compact: String = cmd.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.is_empty() || !compact.len().is_multiple_of(2) {
            return "?".into();
        }
        let Ok(req) = (0..compact.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&compact[i..i + 2], 16))
            .collect::<std::result::Result<Vec<u8>, _>>()
        else {
            return "?".into();
        };
        let Adapter { header_id, cea, .. } = self.adapter;
        let pack = self.car.pack;
        match cea {
            Some(target) if header_id == BMW_TESTER_ID => {
                match pack.module(target).filter(|m| m.link == Link::BmwExtended) {
                    Some(_) if self.faults.contains_key(&target) => self.kwp(target, &req),
                    _ => "NO DATA".into(),
                }
            }
            Some(_) => "NO DATA".into(),
            None if header_id == 0x7DF => self.obd(&req),
            None => {
                let module = pack
                    .modules
                    .iter()
                    .find(|m| matches!(m.link, Link::Can11 { .. }) && m.request_id() == header_id);
                match module {
                    Some(m) if self.faults.contains_key(&m.address) => {
                        let reply = self.uds(m, &req);
                        self.frames(m.response_id(), &reply, Addressing::Normal)
                    }
                    _ => "NO DATA".into(),
                }
            }
        }
    }

    // ---- OBD-II -------------------------------------------------------

    /// The engine module's reply to an OBD-II request.
    fn obd_payload(&mut self, req: &[u8]) -> Option<Vec<u8>> {
        match req {
            [0x01, pid] => self.pid(*pid).map(|data| {
                let mut r = vec![0x41, *pid];
                r.extend(data);
                r
            }),
            // Several PIDs in one request, as J1979 allows on CAN: the
            // reply carries each supported one in turn.
            [0x01, pids @ ..] if self.multi_pid && (2..=6).contains(&pids.len()) => {
                let mut r = vec![0x41];
                for &pid in pids {
                    if let Some(data) = self.pid(pid) {
                        r.push(pid);
                        r.extend(data);
                    }
                }
                (r.len() > 1).then_some(r)
            }
            [0x02, pid, frame] => self.freeze_frame(*pid, *frame),
            [0x03] => Some(dtc_reply(0x43, &self.stored)),
            [0x07] => Some(dtc_reply(0x47, &self.pending)),
            [0x0A] => Some(dtc_reply(0x4A, &self.permanent)),
            [0x04] => {
                // What a real car does on clear: codes and freeze frame go,
                // and every supported monitor starts over. Permanent codes
                // stay until the car proves the fault is fixed.
                self.stored.clear();
                self.pending.clear();
                self.freeze = None;
                self.monitors.1 = self.monitors.0;
                Some(vec![0x44])
            }
            [0x09, 0x00] => Some(vec![0x49, 0x00, 0x54, 0x40, 0x00, 0x00]),
            [0x09, 0x02] => {
                let mut r = vec![0x49, 0x02, 0x01];
                r.extend_from_slice(self.car.vin.as_bytes());
                Some(r)
            }
            _ => None,
        }
    }

    fn obd(&mut self, req: &[u8]) -> String {
        let Some(reply) = self.obd_payload(req) else {
            return "NO DATA".into();
        };
        let mut lines = self.frames(0x7E8, &reply, Addressing::Normal);
        // The transmission also answers the supported-PID query, as on a
        // real car, so multi-module grouping is exercised.
        if req == [0x01, 0x00] {
            lines.push('\r');
            lines.push_str(&self.frames(
                0x7E9,
                &[0x41, 0x00, 0x98, 0x18, 0x80, 0x01],
                Addressing::Normal,
            ));
        }
        lines
    }

    /// Service 02: values captured when the first emissions code set.
    fn freeze_frame(&self, pid: u8, frame: u8) -> Option<Vec<u8>> {
        if frame != 0 {
            return None;
        }
        let mut r = vec![0x42, pid, frame];
        match (&self.freeze, pid) {
            (None, 0x02) => r.extend([0, 0]),
            (None, _) => return None,
            (Some((dtc, _)), 0x02) => r.extend(dtc),
            (Some((_, values)), _) => r.extend(values.get(&pid)?),
        }
        Some(r)
    }

    /// Live values. Idle wobbles; engine-off reads 0 rpm.
    fn pid(&self, pid: u8) -> Option<Vec<u8>> {
        let t = self.t();
        let run = self.running();
        let warm = (t / 120.0).min(1.0);
        let coolant = 40.0 + 65.0 * warm + (t * 0.3).sin();
        let dip = self.stumble_depth();
        let rpm = if run {
            700.0 + 25.0 * (t * 1.7).sin() + 10.0 * (t * 5.3).sin() - 190.0 * dip
        } else {
            0.0
        };
        let word = |v: f64| {
            let v = v.clamp(0.0, 65535.0) as u16;
            v.to_be_bytes().to_vec()
        };
        let byte = |v: f64| vec![v.clamp(0.0, 255.0).round() as u8];
        // Short-term trims make up whatever the learned (long-term) trim no
        // longer covers, so they jump after an adaptation reset.
        let stft = |wave: f64, leak: f64, bank: u8| {
            byte(
                128.0
                    + if run {
                        3.0 * wave + (leak - self.ltft(bank)) * 0.8 + 12.0 * dip
                    } else {
                        0.0
                    } * 1.28,
            )
        };
        Some(match pid {
            // Single-bank engines do not support PIDs 08 and 09.
            0x00 if self.two_banks() => vec![0xBF, 0xBF, 0xB8, 0x13],
            0x00 => vec![0xBE, 0x3F, 0xB8, 0x13],
            0x01 => {
                let mil = if self.stored.is_empty() { 0x00 } else { 0x80 };
                vec![
                    mil | self.stored.len() as u8,
                    0x07,
                    self.monitors.0,
                    self.monitors.1,
                ]
            }
            0x20 => vec![0xA0, 0x07, 0xB0, 0x11],
            0x40 => vec![0xFE, 0xD0, 0x00, 0x10],
            0x04 => byte(
                if run {
                    22.0 + 2.0 * (t * 1.3).sin() + 12.0 * dip
                } else {
                    0.0
                } * 255.0
                    / 100.0,
            ),
            0x05 => byte(coolant + 40.0),
            0x06 => stft((t * 2.1).sin(), self.leak.0, 1),
            0x07 => byte(128.0 + self.ltft(1) * 1.28),
            0x08 if self.two_banks() => stft((t * 1.9).cos(), self.leak.1, 2),
            0x09 if self.two_banks() => byte(128.0 + self.ltft(2) * 1.28),
            0x0B => byte(if run {
                34.0 + 1.5 * (t * 1.1).sin() + 9.0 * dip
            } else {
                101.0
            }),
            0x0C => word(rpm * 4.0),
            0x0D => vec![0],
            0x0E => byte(
                (if run {
                    9.0 + 2.0 * (t * 0.8).sin() - 5.0 * dip
                } else {
                    0.0
                } + 64.0)
                    * 2.0,
            ),
            0x0F => byte(32.0 + 40.0),
            0x10 => word(
                if run {
                    4.1 + 0.3 * (t * 1.4).sin()
                } else {
                    0.0
                } * 100.0,
            ),
            0x11 => byte(if run { 82.0 } else { 0.0 } * 255.0 / 100.0),
            // A misfire sends unburnt oxygen past the sensor: it reads lean.
            0x14 if dip > 0.3 => vec![14, 0xFF],
            0x14 => vec![
                if run {
                    (90.0 + 80.0 * (t * 4.0).sin()) as u8
                } else {
                    0
                },
                0xFF,
            ],
            // Downstream sensor: steady behind a working converter, copying
            // the upstream one behind a worn converter.
            0x15 if self.cat_worn && run => {
                vec![(100.0 + 70.0 * (t * 4.0 + 0.6).sin()) as u8, 0xFF]
            }
            0x15 => vec![if run { 140 } else { 0 }, 0xFF],
            0x1F => word(if run { t } else { 0.0 }),
            0x21 => word(0.0),
            // Distance since codes were cleared: short on the Camry.
            0x31 => word(if self.monitors.1 == 0x25 {
                38.0
            } else {
                1843.0
            }),
            0x2F => byte(61.0 * 255.0 / 100.0),
            // Warm-ups since codes were cleared: few on the Camry.
            0x30 => byte(if self.monitors.1 == 0x25 { 3.0 } else { 112.0 }),
            // Odometer: only newer cars report it over OBD-II.
            0xA6 => {
                let km: f64 = match self.car.id {
                    "mercedes_gle" => 61_240.3,
                    "toyota_camry" => 48_877.0,
                    "honda_accord" => 72_105.6,
                    "jeep_wrangler" => 39_512.8,
                    _ => return None,
                };
                ((km * 10.0) as u32).to_be_bytes().to_vec()
            }
            0x33 => byte(101.0),
            0x3C => word((if run { 520.0 * warm + 20.0 } else { 20.0 } + 40.0) * 10.0),
            0x42 => word((self.volts() + (t * 0.7).sin() * 0.05) * 1000.0),
            0x44 => word(if run { 32768.0 } else { 0.0 }),
            0x46 => byte(18.0 + 40.0),
            0x5C => byte(coolant - 6.0 * warm + 40.0),
            _ => return None,
        })
    }

    // ---- writes shared by KWP2000 and UDS ------------------------------

    /// Applies a record write with the rules a real module enforces.
    /// Returns the negative response code on refusal.
    fn write(&mut self, target: u8, id: u16, data: &[u8]) -> std::result::Result<(), u8> {
        let def = self.car.pack.records_for(target).find(|d| d.id == id);
        match def {
            // Learned values cannot be written, only reset.
            Some(d) if d.kind != RecordKind::Coding => Err(0x31),
            Some(d) if data.len() != d.len() => Err(0x13),
            // Coding needs the engine off.
            Some(_) if self.running() => Err(0x22),
            Some(_) => {
                self.records.insert((target, id), data.to_vec());
                Ok(())
            }
            None => Err(0x31),
        }
    }

    fn identification(&self, target: u8) -> String {
        let name = self
            .car
            .pack
            .module(target)
            .map(|m| m.name)
            .unwrap_or("UNKNOWN");
        format!("SIM {name} 0000000 HW01 SW01")
    }

    // ---- BMW KWP2000 --------------------------------------------------

    fn kwp(&mut self, target: u8, req: &[u8]) -> String {
        let Some(&sid) = req.first() else {
            return "?".into();
        };
        let reply: Vec<u8> = match req {
            [0x21, lid] => match self.records.get(&(target, *lid as u16)) {
                Some(v) => {
                    let mut r = vec![0x61, *lid];
                    r.extend_from_slice(v);
                    r
                }
                None => vec![0x7F, sid, 0x31],
            },
            [0x3B, lid, data @ ..] => match self.write(target, *lid as u16, data) {
                Ok(()) => vec![0x7B, *lid],
                Err(nrc) => vec![0x7F, sid, nrc],
            },
            [0x3E, ..] => vec![0x7E],
            [0x18, 0x02, 0xFF, 0xFF] => {
                let entries = &self.faults[&target];
                let mut r = vec![0x58, entries.len() as u8];
                for e in entries {
                    r.extend_from_slice(e);
                }
                r
            }
            [0x14, 0xFF, 0xFF] => {
                self.faults.entry(target).or_default().clear();
                vec![0x54, 0xFF, 0xFF]
            }
            [0x1A, 0x80] => {
                let mut r = vec![0x5A, 0x80];
                r.extend_from_slice(self.identification(target).as_bytes());
                r
            }
            // Anything else that writes or actuates is refused.
            [0x27 | 0x2E | 0x2F | 0x30 | 0x31, ..] => vec![0x7F, sid, 0x22],
            _ => vec![0x7F, sid, 0x11],
        };
        self.frames(
            0x600 + target as CanId,
            &reply,
            Addressing::Extended(BMW_TESTER_ADDRESS),
        )
    }

    // ---- UDS on 11-bit CAN ----------------------------------------------

    fn uds(&mut self, module: &Module, req: &[u8]) -> Vec<u8> {
        let target = module.address;
        let Some(&sid) = req.first() else {
            return vec![0x7F, 0x00, 0x13];
        };
        match req {
            [0x10, session] => vec![0x50, *session, 0x00, 0x32, 0x01, 0xF4],
            [0x3E, sub] => vec![0x7E, sub & 0x7F],
            [0x19, 0x02, _mask] => {
                let mut r = vec![0x59, 0x02, 0xFF];
                for e in &self.faults[&target] {
                    r.extend_from_slice(e);
                }
                r
            }
            [0x14, ..] => {
                self.faults.entry(target).or_default().clear();
                vec![0x54]
            }
            [0x22, hi, lo] => {
                let did = u16::from_be_bytes([*hi, *lo]);
                let value = match did {
                    0xF187 => Some(format!("SIM-{}-0001", module.name).into_bytes()),
                    0xF189 => Some(b"SW 01.02".to_vec()),
                    0xF190 => Some(self.car.vin.as_bytes().to_vec()),
                    _ => self.records.get(&(target, did)).cloned(),
                };
                match value {
                    Some(v) => {
                        let mut r = vec![0x62, *hi, *lo];
                        r.extend(v);
                        r
                    }
                    None => vec![0x7F, sid, 0x31],
                }
            }
            [0x2E, hi, lo, data @ ..] => {
                match self.write(target, u16::from_be_bytes([*hi, *lo]), data) {
                    Ok(()) => vec![0x6E, *hi, *lo],
                    Err(nrc) => vec![0x7F, sid, nrc],
                }
            }
            // The engine module also answers OBD-II services on its own ID.
            [0x01..=0x0A, ..] if module.is_obd_legislated() && module.request_id() == 0x7E0 => self
                .obd_payload(req)
                .unwrap_or_else(|| vec![0x7F, sid, 0x12]),
            [0x27 | 0x2F | 0x31, ..] => vec![0x7F, sid, 0x22],
            _ => vec![0x7F, sid, 0x11],
        }
    }

    // ---- framing ------------------------------------------------------

    /// Prints a reply the way an ELM327 with headers on does: one line per
    /// CAN frame, Flow Control already handled.
    fn frames(&self, id: CanId, payload: &[u8], addressing: Addressing) -> String {
        let frames = match segment(payload, addressing).expect("simulator reply fits ISO-TP") {
            Segmented::Single(f) => vec![f],
            Segmented::Multi { first, mut sender } => {
                let (mut rest, _) = sender
                    .on_flow_control(FlowControl::CONTINUE)
                    .expect("continue-to-send never fails");
                rest.insert(0, first);
                rest
            }
        };
        frames
            .iter()
            .map(|f| {
                let bytes = f
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                if self.adapter.headers {
                    format!("{id:03X} {bytes}")
                } else {
                    bytes
                }
            })
            .collect::<Vec<_>>()
            .join("\r")
    }
}

/// Two-byte SAE form of a code like `P0420` or `C1525`.
fn sae(code: &str) -> [u8; 2] {
    let b = code.as_bytes();
    let group = match b[0] {
        b'P' => 0,
        b'C' => 1,
        b'B' => 2,
        _ => 3,
    };
    let digit = |i: usize| (b[i] as char).to_digit(16).unwrap_or(0) as u8;
    [
        group << 6 | digit(1) << 4 | digit(2),
        digit(3) << 4 | digit(4),
    ]
}

fn dtc_reply(sid: u8, codes: &[[u8; 2]]) -> Vec<u8> {
    let mut r = vec![sid, codes.len() as u8];
    for c in codes {
        r.extend_from_slice(c);
    }
    r
}

#[async_trait]
impl HardwareTransport for ElmSimulator {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        self.out.clear();
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn send(&mut self, bytes: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        let text = String::from_utf8_lossy(bytes);
        for line in text.split('\r').filter(|l| !l.trim().is_empty()) {
            let reply = self.handle(line);
            self.out.extend(reply.bytes());
        }
        Ok(())
    }

    fn simulator(&mut self) -> Option<&mut ElmSimulator> {
        Some(self)
    }

    async fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        if self.out.is_empty() {
            return Err(TransportError::Timeout(timeout));
        }
        // Hand data back in BLE-sized chunks to exercise reassembly of text.
        let n = self.out.len().min(20);
        Ok(self.out.drain(..n).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_core::obd2::decode_dtc;

    #[test]
    fn sae_round_trips() {
        for code in ["P0171", "P0420", "C1525", "B2785", "U0155", "P0300"] {
            let [hi, lo] = sae(code);
            assert_eq!(decode_dtc(hi, lo), code);
        }
    }

    #[test]
    fn every_demo_car_has_a_pack_and_demo_vin() {
        for car in &SIM_CARS {
            assert!(car.vin.starts_with(protocol_core::vin::DEMO_PREFIX));
            assert_eq!(car.vin.len(), 17);
            let sim = ElmSimulator::with_car(car, SimEngine::Idling);
            assert!(!sim.faults.is_empty(), "{}", car.id);
            for (addr, id) in sim.records.keys() {
                assert!(
                    car.pack.records_for(*addr).any(|d| d.id == *id),
                    "{}: record {addr:02X}/{id:04X} not in pack",
                    car.id
                );
            }
            for addr in sim.faults.keys() {
                assert!(car.pack.module(*addr).is_some());
            }
        }
    }

    #[test]
    fn demo_repairs_change_what_the_checks_see() {
        let mut sim = ElmSimulator::with_car(sim_car("honda_accord").unwrap(), SimEngine::Idling);
        assert!(sim.cat_worn);
        assert!(!sim.apply_demo_repair("P0171"));
        assert!(sim.apply_demo_repair("P0420-00"));
        assert!(!sim.cat_worn);
        let mut sim = ElmSimulator::new(SimEngine::Idling);
        assert!(sim.ltft(1) > 9.0);
        assert!(sim.apply_demo_repair("P0171"));
        assert!(sim.ltft(1) < 1.0 && sim.leak.0 < 1.0);
        let mut sim = ElmSimulator::with_car(sim_car("jeep_wrangler").unwrap(), SimEngine::Idling);
        assert!(sim.volts() < 13.2);
        assert!(sim.apply_demo_repair("P0562"));
        assert!(sim.volts() > 13.5);
    }

    #[test]
    fn jeep_tells_its_story() {
        let car = sim_car("jeep_wrangler").unwrap();
        assert_eq!(car.vin.len(), 17);
        let sim = ElmSimulator::with_car(car, SimEngine::Idling);
        assert_eq!(sim.stored, vec![sae("P0456")]);
        assert_eq!(sim.pending, vec![sae("P0562")]);
        // Park assist is not fitted; every fault has demo text.
        assert!(!sim.faults.contains_key(&jeep_jl::PAM.address));
        for (addr, code) in [(0xE0, "P0456-00"), (0x42, "U3003-16"), (0x24, "C0460-00")] {
            assert!(sim_fault_text("jeep_jl", addr, code)
                .unwrap()
                .starts_with("Demo:"));
        }
    }

    #[test]
    fn demo_stumble_dips_rpm_once() {
        let mut sim = ElmSimulator::new(SimEngine::Idling);
        let rpm = |sim: &ElmSimulator| {
            let b = sim.pid(0x0C).unwrap();
            u16::from_be_bytes([b[0], b[1]]) as f64 / 4.0
        };
        assert_eq!(sim.stumble_depth(), 0.0);
        // Start the stumble 0.6 s ago: the middle of the dip.
        sim.stumble_at = Some(sim.t() - 0.6);
        assert!(sim.stumble_depth() > 0.9);
        assert!(rpm(&sim) < 560.0);
        assert_eq!(sim.pid(0x14).unwrap()[0], 14);
        sim.stumble_at = Some(sim.t() - 5.0);
        assert_eq!(sim.stumble_depth(), 0.0);
        assert!(rpm(&sim) > 640.0);
    }
}
