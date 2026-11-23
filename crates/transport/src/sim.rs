//! A simulated ELM327 plugged into a simulated E70 X5.
//!
//! It answers the same text protocol a real adapter does, so the whole stack
//! (client, parser, ISO-TP, UI) can be developed and tested without a car.
//! Values move over time so gauges have something to show.
//!
//! Everything it reports is invented. The VIN is `SIMULATED0APEX001`.

use crate::{HardwareTransport, Result, TransportError};
use async_trait::async_trait;
use protocol_core::bmw_e70;
use protocol_core::isotp::{segment, Addressing, FlowControl, Segmented};
use protocol_core::snapshot::RecordKind;
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

pub const SIM_VIN: &str = "SIMULATED0APEX001";

/// Descriptions for the simulator's invented BMW fault entries. These are
/// demo text, not real BMW fault meanings.
pub fn sim_fault_text(address: u8, code: &str) -> Option<&'static str> {
    Some(match (address, code) {
        (0x12, "29F3") => "Demo: oil level sensor signal implausible",
        (0x12, "2A82") => "Demo: intake VANOS solenoid, response too slow",
        (0x29, "5E20") => "Demo: wheel speed sensor rear left, signal intermittent",
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

pub struct ElmSimulator {
    connected: bool,
    out: VecDeque<u8>,
    start: Instant,
    engine: SimEngine,
    charger: bool,
    echo: bool,
    headers: bool,
    header_id: u32,
    cea: Option<u8>,
    stored: Vec<[u8; 2]>,
    pending: Vec<[u8; 2]>,
    permanent: Vec<[u8; 2]>,
    /// Module address -> fault memory entries `[hi, lo, status]`.
    faults: BTreeMap<u8, Vec<[u8; 3]>>,
    /// (module address, local id) -> raw record value, for the Time Machine.
    records: BTreeMap<(u8, u8), Vec<u8>>,
    /// Requests seen, for tests.
    pub log: Vec<String>,
}

impl Default for ElmSimulator {
    fn default() -> Self {
        Self::new(SimEngine::Idling)
    }
}

impl ElmSimulator {
    pub fn new(engine: SimEngine) -> Self {
        let mut faults = BTreeMap::new();
        // Invented entries so the fault screens have content.
        faults.insert(
            bmw_e70::DME.address,
            vec![[0x29, 0xF3, 0x24], [0x2A, 0x82, 0x20]],
        );
        faults.insert(bmw_e70::DSC.address, vec![[0x5E, 0x20, 0x24]]);
        faults.insert(bmw_e70::FRM.address, vec![]);
        for m in [
            bmw_e70::JBE,
            bmw_e70::ACSM,
            bmw_e70::EGS,
            bmw_e70::VGSG,
            bmw_e70::CAS,
            bmw_e70::KOMBI,
            bmw_e70::IHKA,
        ] {
            faults.insert(m.address, vec![]);
        }
        Self {
            connected: false,
            out: VecDeque::new(),
            start: Instant::now(),
            engine,
            charger: false,
            echo: true,
            headers: false,
            header_id: 0x7DF,
            cea: None,
            stored: vec![[0x01, 0x71], [0x01, 0x74]],
            pending: vec![[0x03, 0x00]],
            permanent: vec![],
            faults,
            records: initial_records(),
            log: Vec::new(),
        }
    }

    /// A record's current raw value.
    pub fn record(&self, address: u8, lid: u8) -> Option<&[u8]> {
        self.records.get(&(address, lid)).map(Vec::as_slice)
    }

    /// What a successful adaptation reset does: the learned values go back
    /// to zero and the module starts relearning.
    pub fn reset_adaptations(&mut self, address: u8) {
        for def in bmw_e70::records_for(address) {
            if def.kind == RecordKind::Adaptation {
                if let Some(v) = self.records.get_mut(&(address, def.lid)) {
                    v.iter_mut().for_each(|b| *b = 0);
                }
            }
        }
    }

    /// Learned long-term fuel trim for a bank, in percent.
    fn ltft(&self, lid: u8) -> f64 {
        self.record(bmw_e70::DME.address, lid)
            .and_then(|raw| {
                bmw_e70::records_for(bmw_e70::DME.address)
                    .find(|d| d.lid == lid)
                    .and_then(|d| d.number(raw))
            })
            .unwrap_or(0.0)
    }

    pub fn set_engine(&mut self, engine: SimEngine) {
        self.engine = engine;
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
            14.2
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
        if self.echo {
            out.push_str(line.trim());
            out.push('\r');
        }
        out.push_str(&body);
        out.push_str("\r\r>");
        out
    }

    fn handle_at(&mut self, at: &str) -> String {
        let compact: String = at.chars().filter(|c| !c.is_whitespace()).collect();
        match compact.as_str() {
            "Z" => {
                *self = Self {
                    connected: true,
                    start: self.start,
                    engine: self.engine,
                    charger: self.charger,
                    stored: std::mem::take(&mut self.stored),
                    pending: std::mem::take(&mut self.pending),
                    permanent: std::mem::take(&mut self.permanent),
                    faults: std::mem::take(&mut self.faults),
                    records: std::mem::take(&mut self.records),
                    log: std::mem::take(&mut self.log),
                    ..Self::new(self.engine)
                };
                "\rELM327 v1.5".into()
            }
            "I" => "ELM327 v1.5".into(),
            "E0" => {
                self.echo = false;
                "OK".into()
            }
            "E1" => {
                self.echo = true;
                "OK".into()
            }
            "H1" => {
                self.headers = true;
                "OK".into()
            }
            "H0" => {
                self.headers = false;
                "OK".into()
            }
            "RV" => {
                let v = self.volts() + (self.t() * 0.7).sin() * 0.05;
                format!("{v:.1}V")
            }
            "DP" => "ISO 15765-4 (CAN 11/500)".into(),
            "CEA" => {
                self.cea = None;
                "OK".into()
            }
            c if c.starts_with("CEA") => match u8::from_str_radix(&c[3..], 16) {
                Ok(a) => {
                    self.cea = Some(a);
                    "OK".into()
                }
                Err(_) => "?".into(),
            },
            c if c.starts_with("SH") => match u32::from_str_radix(&c[2..], 16) {
                Ok(id) => {
                    self.header_id = id;
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
        match self.cea {
            Some(target) if self.header_id == bmw_e70::TESTER_ID => self.bmw(target, &req),
            Some(_) => "NO DATA".into(),
            None => self.obd(&req),
        }
    }

    // ---- OBD-II -------------------------------------------------------

    fn obd(&mut self, req: &[u8]) -> String {
        let reply = match req {
            [0x01, pid] => self.pid(*pid).map(|data| {
                let mut r = vec![0x41, *pid];
                r.extend(data);
                r
            }),
            [0x03] => Some(dtc_reply(0x43, &self.stored)),
            [0x07] => Some(dtc_reply(0x47, &self.pending)),
            [0x0A] => Some(dtc_reply(0x4A, &self.permanent)),
            [0x04] => {
                self.stored.clear();
                self.pending.clear();
                Some(vec![0x44])
            }
            [0x09, 0x00] => Some(vec![0x49, 0x00, 0x54, 0x40, 0x00, 0x00]),
            [0x09, 0x02] => {
                let mut r = vec![0x49, 0x02, 0x01];
                r.extend_from_slice(SIM_VIN.as_bytes());
                Some(r)
            }
            _ => None,
        };
        let Some(reply) = reply else {
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

    /// Live values. Idle wobbles; engine-off reads 0 rpm.
    fn pid(&self, pid: u8) -> Option<Vec<u8>> {
        let t = self.t();
        let run = self.running();
        let warm = (t / 120.0).min(1.0);
        let coolant = 40.0 + 65.0 * warm + (t * 0.3).sin();
        let rpm = if run {
            700.0 + 25.0 * (t * 1.7).sin() + 10.0 * (t * 5.3).sin()
        } else {
            0.0
        };
        let word = |v: f64| {
            let v = v.clamp(0.0, 65535.0) as u16;
            v.to_be_bytes().to_vec()
        };
        let byte = |v: f64| vec![v.clamp(0.0, 255.0).round() as u8];
        Some(match pid {
            0x00 => vec![0xBF, 0xBF, 0xB8, 0x13],
            0x01 => {
                let mil = if self.stored.is_empty() { 0x00 } else { 0x80 };
                vec![mil | self.stored.len() as u8, 0x07, 0x65, 0x04]
            }
            0x20 => vec![0xA0, 0x07, 0xB0, 0x11],
            0x40 => vec![0xFE, 0xD0, 0x00, 0x10],
            0x04 => byte(
                if run {
                    22.0 + 2.0 * (t * 1.3).sin()
                } else {
                    0.0
                } * 255.0
                    / 100.0,
            ),
            0x05 => byte(coolant + 40.0),
            // Short-term trims make up whatever the learned (long-term)
            // trim no longer covers, so they jump after an adaptation reset.
            0x06 => byte(
                128.0
                    + if run {
                        3.0 * (t * 2.1).sin() + (SIM_LEAK_B1 - self.ltft(0x01)) * 0.8
                    } else {
                        0.0
                    } * 1.28,
            ),
            0x07 => byte(128.0 + self.ltft(0x01) * 1.28),
            0x08 => byte(
                128.0
                    + if run {
                        3.0 * (t * 1.9).cos() + (SIM_LEAK_B2 - self.ltft(0x02)) * 0.8
                    } else {
                        0.0
                    } * 1.28,
            ),
            0x09 => byte(128.0 + self.ltft(0x02) * 1.28),
            0x0B => byte(if run {
                34.0 + 1.5 * (t * 1.1).sin()
            } else {
                101.0
            }),
            0x0C => word(rpm * 4.0),
            0x0D => vec![0],
            0x0E => byte(
                (if run {
                    9.0 + 2.0 * (t * 0.8).sin()
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
            0x14 => vec![
                if run {
                    (90.0 + 80.0 * (t * 4.0).sin()) as u8
                } else {
                    0
                },
                0xFF,
            ],
            0x15 => vec![if run { 140 } else { 0 }, 0xFF],
            0x1F => word(if run { t } else { 0.0 }),
            0x21 => word(0.0),
            0x2F => byte(61.0 * 255.0 / 100.0),
            0x31 => word(1843.0),
            0x33 => byte(101.0),
            0x3C => word((if run { 520.0 * warm + 20.0 } else { 20.0 } + 40.0) * 10.0),
            0x42 => word((self.volts() + (t * 0.7).sin() * 0.05) * 1000.0),
            0x44 => word(if run { 32768.0 } else { 0.0 }),
            0x46 => byte(18.0 + 40.0),
            0x5C => byte(coolant - 6.0 * warm + 40.0),
            _ => return None,
        })
    }

    // ---- BMW KWP2000 --------------------------------------------------

    fn bmw(&mut self, target: u8, req: &[u8]) -> String {
        if !self.faults.contains_key(&target) {
            return "NO DATA".into();
        }
        let Some(&sid) = req.first() else {
            return "?".into();
        };
        let running = self.running();
        let reply: Vec<u8> = match req {
            [0x21, lid] => match self.records.get(&(target, *lid)) {
                Some(v) => {
                    let mut r = vec![0x61, *lid];
                    r.extend_from_slice(v);
                    r
                }
                None => vec![0x7F, sid, 0x31],
            },
            [0x3B, lid, data @ ..] => {
                let def = bmw_e70::records_for(target).find(|d| d.lid == *lid);
                match def {
                    // Learned values cannot be written, only reset.
                    Some(d) if d.kind != RecordKind::Coding => vec![0x7F, sid, 0x31],
                    Some(d) if data.len() != d.len() => vec![0x7F, sid, 0x13],
                    // Coding needs the engine off.
                    Some(_) if running => vec![0x7F, sid, 0x22],
                    Some(_) => {
                        self.records.insert((target, *lid), data.to_vec());
                        vec![0x7B, *lid]
                    }
                    None => vec![0x7F, sid, 0x31],
                }
            }
            _ => self.bmw_fixed(target, req),
        };
        self.frames(
            0x600 + target as u32,
            &reply,
            Addressing::Extended(bmw_e70::TESTER_ADDRESS),
        )
    }

    fn bmw_fixed(&mut self, target: u8, req: &[u8]) -> Vec<u8> {
        let sid = req[0];
        let entries = self.faults.entry(target).or_default();
        match req {
            [0x3E, ..] => vec![0x7E],
            [0x18, 0x02, 0xFF, 0xFF] => {
                let mut r = vec![0x58, entries.len() as u8];
                for e in entries.iter() {
                    r.extend_from_slice(e);
                }
                r
            }
            [0x14, 0xFF, 0xFF] => {
                entries.clear();
                vec![0x54, 0xFF, 0xFF]
            }
            [0x1A, 0x80] => {
                let name = bmw_e70::module_at(target)
                    .map(|m| m.name)
                    .unwrap_or("UNKNOWN");
                let mut r = vec![0x5A, 0x80];
                r.extend_from_slice(format!("SIM {name} 0000000 HW01 SW01").as_bytes());
                r
            }
            // Anything else that writes or actuates is refused.
            [0x27 | 0x2E | 0x2F | 0x30 | 0x31, ..] => vec![0x7F, sid, 0x22],
            _ => vec![0x7F, sid, 0x11],
        }
    }

    // ---- framing ------------------------------------------------------

    /// Prints a reply the way an ELM327 with headers on does: one line per
    /// CAN frame, Flow Control already handled.
    fn frames(&self, id: u32, payload: &[u8], addressing: Addressing) -> String {
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
                if self.headers {
                    format!("{id:03X} {bytes}")
                } else {
                    bytes
                }
            })
            .collect::<Vec<_>>()
            .join("\r")
    }
}

/// The demo car's underlying fuel error per bank (a small vacuum leak), in
/// percent. Learned trims sit near this; short-term trims cover the rest.
const SIM_LEAK_B1: f64 = 9.4;
const SIM_LEAK_B2: f64 = 8.6;

/// Today's values of the demo car's Time Machine records.
fn initial_records() -> BTreeMap<(u8, u8), Vec<u8>> {
    let (dme, egs, frm, cas, kombi) = (
        bmw_e70::DME.address,
        bmw_e70::EGS.address,
        bmw_e70::FRM.address,
        bmw_e70::CAS.address,
        bmw_e70::KOMBI.address,
    );
    let w = |v: i16| v.to_be_bytes().to_vec();
    BTreeMap::from([
        ((dme, 0x01), w(940)),
        ((dme, 0x02), w(860)),
        ((dme, 0x03), w(18)),
        ((dme, 0x04), w(-24)),
        ((dme, 0x05), w(62)),
        ((dme, 0x10), vec![210]),
        ((dme, 0x11), vec![0x00]),
        ((egs, 0x30), w(84)),
        ((egs, 0x31), w(41)),
        ((egs, 0x32), w(1843)),
        ((frm, 0x20), vec![0x01]),
        ((frm, 0x21), vec![5]),
        ((frm, 0x22), vec![0x00]),
        ((cas, 0x40), vec![0x01]),
        ((cas, 0x41), vec![0x01]),
        ((kombi, 0x50), vec![0x00]),
        ((kombi, 0x51), vec![0x00]),
    ])
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
