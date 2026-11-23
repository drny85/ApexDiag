//! SAE J1979 generic OBD-II, the part every car since 1996 (US) speaks.
//!
//! Requests go to the functional broadcast ID `0x7DF`; each engine-related
//! module answers from `0x7E8`-`0x7EF`. All decoding here works on the
//! reassembled ISO-TP payload, e.g. `41 0C 1A F8` for engine speed.

use crate::{CanId, ProtocolError};
use serde::Serialize;

/// Functional (broadcast) request ID for 11-bit OBD-II.
pub const FUNCTIONAL_REQUEST: CanId = 0x7DF;
/// Reply IDs: 0x7E8 is normally the engine module.
pub const RESPONSE_IDS: std::ops::RangeInclusive<CanId> = 0x7E8..=0x7EF;

/// J1979 service identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum Service {
    /// Live data.
    CurrentData = 0x01,
    /// Values captured when an emissions fault was confirmed.
    FreezeFrame = 0x02,
    /// Confirmed emissions codes (these turn the check-engine light on).
    StoredDtcs = 0x03,
    /// Clears codes and resets readiness monitors.
    ClearDtcs = 0x04,
    /// Codes seen this drive cycle but not yet confirmed.
    PendingDtcs = 0x07,
    /// VIN and calibration IDs.
    VehicleInfo = 0x09,
    /// Codes that only clear themselves once the car proves the fault fixed.
    PermanentDtcs = 0x0A,
}

impl Service {
    pub const fn response_sid(self) -> u8 {
        self as u8 + 0x40
    }
}

/// How a PID turns bytes into a number.
#[derive(Debug, Clone, Copy)]
pub enum Formula {
    /// A
    A,
    /// A - 40 (temperatures)
    AMinus40,
    /// A * 100 / 255 (percentages)
    Percent,
    /// (A - 128) * 100 / 128 (fuel trims)
    Trim,
    /// (256A + B) / 4 (RPM)
    Rpm,
    /// A / 2 - 64 (timing advance)
    Timing,
    /// (256A + B) / 100 (mass air flow)
    Maf,
    /// 256A + B
    Word,
    /// (256A + B) / 1000 (volts)
    Millivolts,
    /// A * 3 (fuel pressure, kPa)
    TimesThree,
    /// A / 200 (oxygen sensor volts)
    O2Volts,
    /// (256A + B) / 10 - 40 (catalyst temperature)
    CatTemp,
    /// (256A + B) * 100 / 255 (absolute load)
    WordPercent,
    /// (256A + B) / 32768 (lambda)
    Lambda,
}

impl Formula {
    pub fn bytes_needed(self) -> usize {
        match self {
            Self::A
            | Self::AMinus40
            | Self::Percent
            | Self::Trim
            | Self::Timing
            | Self::TimesThree
            | Self::O2Volts => 1,
            _ => 2,
        }
    }

    pub fn apply(self, d: &[u8]) -> Option<f64> {
        let a = *d.first()? as f64;
        let word = || -> Option<f64> { Some(a * 256.0 + *d.get(1)? as f64) };
        Some(match self {
            Self::A => a,
            Self::AMinus40 => a - 40.0,
            Self::Percent => a * 100.0 / 255.0,
            Self::Trim => (a - 128.0) * 100.0 / 128.0,
            Self::Rpm => word()? / 4.0,
            Self::Timing => a / 2.0 - 64.0,
            Self::Maf => word()? / 100.0,
            Self::Word => word()?,
            Self::Millivolts => word()? / 1000.0,
            Self::TimesThree => a * 3.0,
            Self::O2Volts => a / 200.0,
            Self::CatTemp => word()? / 10.0 - 40.0,
            Self::WordPercent => word()? * 100.0 / 255.0,
            Self::Lambda => word()? / 32768.0,
        })
    }
}

/// A live-data parameter with everything the UI needs to show and explain it.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct PidInfo {
    pub pid: u8,
    pub name: &'static str,
    pub unit: &'static str,
    /// Gauge range.
    pub min: f64,
    pub max: f64,
    /// What it measures and why it matters.
    pub explain: &'static str,
    #[serde(skip)]
    pub formula: Formula,
}

macro_rules! pid {
    ($pid:expr, $name:expr, $unit:expr, $min:expr, $max:expr, $f:ident, $explain:expr) => {
        PidInfo {
            pid: $pid,
            name: $name,
            unit: $unit,
            min: $min,
            max: $max,
            formula: Formula::$f,
            explain: $explain,
        }
    };
}

/// Service 01 PIDs the app understands.
pub const PIDS: &[PidInfo] = &[
    pid!(0x04, "Engine load", "%", 0.0, 100.0, Percent, "How hard the engine is working compared with its maximum at this speed. High at idle can mean a drag on the engine."),
    pid!(0x05, "Coolant temperature", "°C", -40.0, 130.0, AMinus40, "Engine coolant temperature. Most engines settle at 85-105 °C; some BMWs run hotter by design. Low readings after warm-up suggest a stuck-open thermostat."),
    pid!(0x06, "Short-term fuel trim B1", "%", -25.0, 25.0, Trim, "Instant fuel correction on bank 1 (the bank with cylinder 1; the only bank on an inline engine). Positive means the engine is adding fuel because it sees too much air."),
    pid!(0x07, "Long-term fuel trim B1", "%", -25.0, 25.0, Trim, "Learned fuel correction on bank 1. Beyond about ±10% points to a vacuum leak, weak fuel delivery or a dirty air-flow sensor."),
    pid!(0x08, "Short-term fuel trim B2", "%", -25.0, 25.0, Trim, "Instant fuel correction on bank 2 (V engines only)."),
    pid!(0x09, "Long-term fuel trim B2", "%", -25.0, 25.0, Trim, "Learned fuel correction on bank 2. Both banks lean together usually means a shared cause such as a vacuum leak or the crankcase vent valve."),
    pid!(0x0A, "Fuel pressure", "kPa", 0.0, 765.0, TimesThree, "Fuel rail gauge pressure, where the engine reports it."),
    pid!(0x0B, "Intake manifold pressure", "kPa", 0.0, 255.0, A, "Absolute pressure in the intake. Around 30-40 kPa at idle on a healthy engine; near 100 kPa at full throttle."),
    pid!(0x0C, "Engine speed", "rpm", 0.0, 7000.0, Rpm, "Crankshaft speed. Also used as a safety interlock: many service functions need this at 0."),
    pid!(0x0D, "Vehicle speed", "km/h", 0.0, 250.0, A, "Road speed. Every actuator test requires this to be exactly 0."),
    pid!(0x0E, "Ignition timing", "°", -64.0, 63.5, Timing, "Spark advance before top dead centre. Pulling back sharply under load can mean knock."),
    pid!(0x0F, "Intake air temperature", "°C", -40.0, 100.0, AMinus40, "Temperature of air entering the engine."),
    pid!(0x10, "Air flow rate", "g/s", 0.0, 655.35, Maf, "Mass of air entering the engine. Roughly 1-1.5 g/s per litre of engine size at idle; low readings at load suggest a dirty sensor."),
    pid!(0x11, "Throttle position", "%", 0.0, 100.0, Percent, "Throttle plate opening. On some engines (BMW Valvetronic, for example) it stays mostly open because the valves control airflow."),
    pid!(0x14, "O2 sensor B1S1", "V", 0.0, 1.275, O2Volts, "Upstream oxygen sensor, bank 1. It should switch quickly between about 0.1 and 0.9 V."),
    pid!(0x15, "O2 sensor B1S2", "V", 0.0, 1.275, O2Volts, "Downstream oxygen sensor, bank 1. A steady signal means the catalytic converter is working."),
    pid!(0x1F, "Run time since start", "s", 0.0, 65535.0, Word, "Seconds since the engine was started."),
    pid!(0x21, "Distance with MIL on", "km", 0.0, 65535.0, Word, "Distance driven with the check-engine light on."),
    pid!(0x2F, "Fuel level", "%", 0.0, 100.0, Percent, "Fuel tank level."),
    pid!(0x31, "Distance since codes cleared", "km", 0.0, 65535.0, Word, "Distance since fault codes were last cleared. A low number when buying a used car is a warning sign."),
    pid!(0x33, "Barometric pressure", "kPa", 0.0, 255.0, A, "Outside air pressure."),
    pid!(0x3C, "Catalyst temperature B1S1", "°C", -40.0, 1000.0, CatTemp, "Estimated catalytic converter temperature, bank 1."),
    pid!(0x42, "Module voltage", "V", 0.0, 18.0, Millivolts, "Supply voltage at the engine module. Below 12.2 V with the engine off means a weak or flat battery; 13.8-14.8 V when running means charging works."),
    pid!(0x43, "Absolute load", "%", 0.0, 400.0, WordPercent, "Air per intake stroke as a percentage of the engine's capacity."),
    pid!(0x44, "Commanded lambda", "λ", 0.0, 2.0, Lambda, "Air-fuel ratio the engine is aiming for. 1.0 is stoichiometric."),
    pid!(0x45, "Relative throttle", "%", 0.0, 100.0, Percent, "Throttle position relative to its learned closed position."),
    pid!(0x46, "Ambient air temperature", "°C", -40.0, 60.0, AMinus40, "Outside temperature."),
    pid!(0x5C, "Oil temperature", "°C", -40.0, 210.0, AMinus40, "Engine oil temperature."),
];

/// Looks up a PID definition.
pub fn pid_info(pid: u8) -> Option<&'static PidInfo> {
    PIDS.iter().find(|p| p.pid == pid)
}

/// Builds a Service 01 request for one PID.
pub fn current_data_request(pid: u8) -> [u8; 2] {
    [Service::CurrentData as u8, pid]
}

/// Decodes a Service 01 reply payload (`41 PID A B ...`).
pub fn decode_current_data(payload: &[u8]) -> Result<(u8, f64), ProtocolError> {
    let [sid, pid, data @ ..] = payload else {
        return Err(ProtocolError::MalformedFrame("live data reply too short"));
    };
    if *sid != Service::CurrentData.response_sid() {
        return Err(ProtocolError::UnexpectedFrame);
    }
    let info = pid_info(*pid).ok_or(ProtocolError::NotImplemented)?;
    if data.len() < info.formula.bytes_needed() {
        return Err(ProtocolError::MalformedFrame(
            "live data reply missing bytes",
        ));
    }
    let value = info
        .formula
        .apply(data)
        .ok_or(ProtocolError::MalformedFrame(
            "live data reply missing bytes",
        ))?;
    Ok((*pid, value))
}

/// Decodes a "PIDs supported" bitmap reply (`41 00 A B C D`, `41 20 ...`).
pub fn decode_supported_pids(payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    let [sid, base, a, b, c, d, ..] = *payload else {
        return Err(ProtocolError::MalformedFrame(
            "supported-PID reply too short",
        ));
    };
    if sid != Service::CurrentData.response_sid() || base % 0x20 != 0 {
        return Err(ProtocolError::UnexpectedFrame);
    }
    let bits = u32::from_be_bytes([a, b, c, d]);
    Ok((0..32)
        .filter(|i| bits & (1 << (31 - i)) != 0)
        .map(|i| base + 1 + i as u8)
        .collect())
}

/// Monitor status (PID 01 01): check-engine light and confirmed code count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MilStatus {
    pub mil_on: bool,
    pub dtc_count: u8,
}

pub fn decode_mil_status(payload: &[u8]) -> Result<MilStatus, ProtocolError> {
    match payload {
        [0x41, 0x01, a, ..] => Ok(MilStatus {
            mil_on: a & 0x80 != 0,
            dtc_count: a & 0x7F,
        }),
        _ => Err(ProtocolError::MalformedFrame("not a monitor status reply")),
    }
}

/// One emissions readiness monitor (service 01, PID 01, bytes B-D).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Monitor {
    pub id: &'static str,
    pub name: &'static str,
    /// The car has this monitor.
    pub supported: bool,
    /// The self-test has finished since codes were last cleared.
    pub complete: bool,
    pub explain: &'static str,
}

/// Emissions readiness: what an inspection station checks. Monitors that
/// are supported but not complete usually mean codes were cleared recently
/// or the drive cycle has not been finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Readiness {
    pub mil_on: bool,
    pub dtc_count: u8,
    pub compression_ignition: bool,
    pub monitors: Vec<Monitor>,
}

impl Readiness {
    pub fn incomplete(&self) -> usize {
        self.monitors
            .iter()
            .filter(|m| m.supported && !m.complete)
            .count()
    }
}

const CONTINUOUS: [(&str, &str, &str); 3] = [
    (
        "misfire",
        "Misfire",
        "Watches crankshaft speed for misfiring cylinders. Runs all the time.",
    ),
    (
        "fuel",
        "Fuel system",
        "Checks that fuel trims stay within limits. Runs all the time.",
    ),
    (
        "components",
        "Comprehensive components",
        "Checks sensors and actuators for open or short circuits. Runs all the time.",
    ),
];

const SPARK: [(&str, &str, &str); 8] = [
    ("catalyst", "Catalyst", "Compares the oxygen sensors before and after the catalytic converter to judge how well it stores oxygen."),
    ("heated_catalyst", "Heated catalyst", "Checks the catalyst heater, on cars that have one."),
    ("evap", "Evaporative system", "Leak-tests the fuel tank vapour system. Often the last to complete: it needs a cold start and a part-full tank."),
    ("secondary_air", "Secondary air", "Checks the pump that blows air into the exhaust during cold starts."),
    ("ac_refrigerant", "A/C refrigerant", "Obsolete; almost never supported."),
    ("o2_sensor", "Oxygen sensors", "Checks how fast the oxygen sensors switch."),
    ("o2_heater", "Oxygen sensor heaters", "Checks the heaters that bring the oxygen sensors up to temperature."),
    ("egr", "EGR / VVT", "Checks exhaust gas recirculation, or variable valve timing on cars without EGR."),
];

/// Decodes PID 01 into readiness monitors (petrol layout; diesel monitors
/// are reported by their raw position).
pub fn decode_readiness(payload: &[u8]) -> Result<Readiness, ProtocolError> {
    let [0x41, 0x01, a, b, c, d, ..] = *payload else {
        return Err(ProtocolError::MalformedFrame("not a monitor status reply"));
    };
    let mut monitors = Vec::new();
    for (i, (id, name, explain)) in CONTINUOUS.iter().enumerate() {
        let supported = b & (1 << i) != 0;
        monitors.push(Monitor {
            id,
            name,
            supported,
            complete: supported && b & (1 << (i + 4)) == 0,
            explain,
        });
    }
    for (i, (id, name, explain)) in SPARK.iter().enumerate() {
        let supported = c & (1 << i) != 0;
        monitors.push(Monitor {
            id,
            name,
            supported,
            complete: supported && d & (1 << i) == 0,
            explain,
        });
    }
    Ok(Readiness {
        mil_on: a & 0x80 != 0,
        dtc_count: a & 0x7F,
        compression_ignition: b & 0x08 != 0,
        monitors,
    })
}

/// Service 02 request: `pid` as captured in freeze frame 0.
pub fn freeze_frame_request(pid: u8) -> [u8; 3] {
    [Service::FreezeFrame as u8, pid, 0x00]
}

/// Decodes a freeze-frame reply (`42 pid frame data...`) with the same
/// formulas as live data.
pub fn decode_freeze_frame(payload: &[u8]) -> Result<(u8, f64), ProtocolError> {
    match payload {
        [0x42, pid, _frame, data @ ..] => {
            let mut live = vec![0x41, *pid];
            live.extend_from_slice(data);
            decode_current_data(&live)
        }
        _ => Err(ProtocolError::MalformedFrame("not a freeze-frame reply")),
    }
}

/// The DTC that caused freeze frame 0 (service 02, PID 02).
pub fn decode_freeze_frame_dtc(payload: &[u8]) -> Result<Option<String>, ProtocolError> {
    match payload {
        [0x42, 0x02, _frame, 0, 0, ..] => Ok(None),
        [0x42, 0x02, _frame, hi, lo, ..] => Ok(Some(decode_dtc(*hi, *lo))),
        _ => Err(ProtocolError::MalformedFrame(
            "not a freeze-frame DTC reply",
        )),
    }
}

/// A generic OBD-II trouble code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dtc {
    /// `P0171`, `U0100`, ...
    pub code: String,
    /// Plain-language description, if known.
    pub description: Option<&'static str>,
}

/// Decodes the two-byte DTC encoding: the top two bits pick the system
/// letter, the next two the first digit, then three hex digits.
pub fn decode_dtc(hi: u8, lo: u8) -> String {
    let letter = ['P', 'C', 'B', 'U'][(hi >> 6) as usize];
    format!("{letter}{}{:X}{:02X}", (hi >> 4) & 0x03, hi & 0x0F, lo)
}

/// Decodes a Service 03 / 07 / 0A reply (`43 NN [hi lo]*` on CAN).
pub fn decode_dtcs(service: Service, payload: &[u8]) -> Result<Vec<Dtc>, ProtocolError> {
    let (&sid, rest) = payload
        .split_first()
        .ok_or(ProtocolError::MalformedFrame("empty DTC reply"))?;
    if sid != service.response_sid() {
        return Err(ProtocolError::UnexpectedFrame);
    }
    // On CAN the first byte is the number of codes.
    let (&count, pairs) = rest
        .split_first()
        .ok_or(ProtocolError::MalformedFrame("DTC reply missing count"))?;
    Ok(pairs
        .as_chunks::<2>()
        .0
        .iter()
        .take(count as usize)
        .filter(|p| **p != [0, 0])
        .map(|p| {
            let code = decode_dtc(p[0], p[1]);
            let description = describe_dtc(&code);
            Dtc { code, description }
        })
        .collect())
}

/// Builds the request for a VIN read (09 02).
pub const VIN_REQUEST: [u8; 2] = [Service::VehicleInfo as u8, 0x02];

/// Decodes a VIN reply (`49 02 01` + 17 ASCII characters).
pub fn decode_vin(payload: &[u8]) -> Result<String, ProtocolError> {
    let data = match payload {
        [0x49, 0x02, _count, rest @ ..] => rest,
        _ => return Err(ProtocolError::MalformedFrame("not a VIN reply")),
    };
    let vin: String = data
        .iter()
        .filter(|b| b.is_ascii_alphanumeric())
        .map(|&b| b as char)
        .collect();
    if vin.len() < 17 {
        return Err(ProtocolError::MalformedFrame(
            "VIN shorter than 17 characters",
        ));
    }
    Ok(vin[vin.len() - 17..].to_string())
}

/// Descriptions for common generic codes, focused on what the E70's N52 /
/// N62 / N55 engines tend to log. A full database comes later.
pub fn describe_dtc(code: &str) -> Option<&'static str> {
    Some(match code {
        "P0011" => "Intake camshaft timing over-advanced, bank 1 (VANOS)",
        "P0012" => "Intake camshaft timing over-retarded, bank 1 (VANOS)",
        "P0014" => "Exhaust camshaft timing over-advanced, bank 1 (VANOS)",
        "P0015" => "Exhaust camshaft timing over-retarded, bank 1 (VANOS)",
        "P0016" => "Crankshaft / intake camshaft correlation, bank 1",
        "P0101" => "Mass air flow sensor range / performance",
        "P0102" => "Mass air flow sensor circuit low",
        "P0113" => "Intake air temperature sensor circuit high",
        "P0115" => "Engine coolant temperature sensor circuit",
        "P0128" => "Coolant temperature below thermostat regulating temperature",
        "P0130" => "O2 sensor circuit, bank 1 sensor 1",
        "P0133" => "O2 sensor slow response, bank 1 sensor 1",
        "P0135" => "O2 sensor heater circuit, bank 1 sensor 1",
        "P0141" => "O2 sensor heater circuit, bank 1 sensor 2",
        "P0155" => "O2 sensor heater circuit, bank 2 sensor 1",
        "P0171" => "System too lean, bank 1",
        "P0172" => "System too rich, bank 1",
        "P0174" => "System too lean, bank 2",
        "P0175" => "System too rich, bank 2",
        "P0300" => "Random / multiple cylinder misfire detected",
        "P0301" => "Cylinder 1 misfire detected",
        "P0302" => "Cylinder 2 misfire detected",
        "P0303" => "Cylinder 3 misfire detected",
        "P0304" => "Cylinder 4 misfire detected",
        "P0305" => "Cylinder 5 misfire detected",
        "P0306" => "Cylinder 6 misfire detected",
        "P0307" => "Cylinder 7 misfire detected",
        "P0308" => "Cylinder 8 misfire detected",
        "P0335" => "Crankshaft position sensor circuit",
        "P0340" => "Camshaft position sensor circuit, bank 1",
        "P0420" => "Catalyst efficiency below threshold, bank 1",
        "P0430" => "Catalyst efficiency below threshold, bank 2",
        "P0441" => "Evaporative system incorrect purge flow",
        "P0455" => "Evaporative system large leak detected",
        "P0456" => "Evaporative system very small leak detected",
        "P0500" => "Vehicle speed sensor",
        "P0505" => "Idle air control system",
        "P0597" => "Thermostat heater control circuit open",
        "P0598" => "Thermostat heater control circuit low",
        "P0600" => "Serial communication link",
        "P0606" => "Control module processor fault",
        "P0700" => "Transmission control system (read the EGS module for details)",
        "P1000" => "Readiness monitors not yet complete (manufacturer-specific)",
        "P2096" => "Post-catalyst fuel trim too lean, bank 1",
        "P2097" => "Post-catalyst fuel trim too rich, bank 1",
        "U0100" => "Lost communication with engine module",
        "U0101" => "Lost communication with transmission module",
        "U0121" => "Lost communication with ABS module",
        "U0155" => "Lost communication with instrument cluster",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn decodes_rpm_speed_coolant() {
        assert_eq!(
            decode_current_data(&[0x41, 0x0C, 0x1A, 0xF8]).unwrap(),
            (0x0C, 1726.0)
        );
        assert_eq!(
            decode_current_data(&[0x41, 0x0D, 0x00]).unwrap(),
            (0x0D, 0.0)
        );
        assert_eq!(
            decode_current_data(&[0x41, 0x05, 0x87]).unwrap(),
            (0x05, 95.0)
        );
    }

    #[test]
    fn decodes_trims_and_voltage() {
        let (_, trim) = decode_current_data(&[0x41, 0x07, 0x90]).unwrap();
        assert!(approx(trim, 12.5));
        let (_, v) = decode_current_data(&[0x41, 0x42, 0x31, 0x2E]).unwrap();
        assert!(approx(v, 12.59));
    }

    #[test]
    fn rejects_short_and_foreign_replies() {
        assert!(decode_current_data(&[0x41, 0x0C, 0x1A]).is_err());
        assert!(decode_current_data(&[0x43, 0x0C, 0x1A, 0xF8]).is_err());
    }

    #[test]
    fn supported_pid_bitmap() {
        // BE 3E B8 11: a common real-world reply.
        let pids = decode_supported_pids(&[0x41, 0x00, 0xBE, 0x3E, 0xB8, 0x11]).unwrap();
        assert_eq!(&pids[..6], &[0x01, 0x03, 0x04, 0x05, 0x06, 0x07]);
        assert!(pids.contains(&0x0C));
        assert!(
            pids.contains(&0x20),
            "bit for 0x20 means 'more PIDs supported'"
        );
    }

    #[test]
    fn dtc_encoding() {
        assert_eq!(decode_dtc(0x01, 0x71), "P0171");
        assert_eq!(decode_dtc(0x41, 0x23), "C0123");
        assert_eq!(decode_dtc(0x80, 0x01), "B0001");
        assert_eq!(decode_dtc(0xC1, 0x00), "U0100");
        assert_eq!(decode_dtc(0x34, 0x20), "P3420");
    }

    #[test]
    fn decodes_stored_codes() {
        let dtcs = decode_dtcs(Service::StoredDtcs, &[0x43, 0x02, 0x01, 0x71, 0x01, 0x74]).unwrap();
        assert_eq!(dtcs.len(), 2);
        assert_eq!(dtcs[0].code, "P0171");
        assert_eq!(dtcs[0].description, Some("System too lean, bank 1"));
        assert!(decode_dtcs(Service::StoredDtcs, &[0x43, 0x00])
            .unwrap()
            .is_empty());
        assert!(decode_dtcs(Service::PendingDtcs, &[0x43, 0x00]).is_err());
    }

    #[test]
    fn decodes_vin_and_mil() {
        let mut payload = vec![0x49, 0x02, 0x01];
        payload.extend_from_slice(b"SIMULATED0APEX001");
        assert_eq!(decode_vin(&payload).unwrap(), "SIMULATED0APEX001");
        assert_eq!(
            decode_mil_status(&[0x41, 0x01, 0x82, 0x07, 0x65, 0x04]).unwrap(),
            MilStatus {
                mil_on: true,
                dtc_count: 2
            }
        );
    }

    #[test]
    fn every_pid_has_an_explanation() {
        for p in PIDS {
            assert!(!p.explain.is_empty() && p.max > p.min, "{:02X}", p.pid);
        }
    }

    #[test]
    fn decodes_readiness_monitors() {
        // MIL on, 2 codes; misfire/fuel/components supported, components
        // incomplete; catalyst, evap, O2, O2 heater, EGR supported; evap
        // and catalyst incomplete.
        let r = decode_readiness(&[0x41, 0x01, 0x82, 0x47, 0xE5, 0x05]).unwrap();
        assert!(r.mil_on);
        assert_eq!(r.dtc_count, 2);
        let get = |id: &str| r.monitors.iter().find(|m| m.id == id).unwrap().clone();
        assert!(get("misfire").supported && get("misfire").complete);
        assert!(get("components").supported && !get("components").complete);
        assert!(get("catalyst").supported && !get("catalyst").complete);
        assert!(get("evap").supported && !get("evap").complete);
        assert!(!get("secondary_air").supported);
        assert!(get("egr").supported && get("egr").complete);
        assert_eq!(r.incomplete(), 3);
    }

    #[test]
    fn decodes_freeze_frame() {
        assert_eq!(freeze_frame_request(0x0C), [0x02, 0x0C, 0x00]);
        let (pid, rpm) = decode_freeze_frame(&[0x42, 0x0C, 0x00, 0x21, 0x70]).unwrap();
        assert_eq!(pid, 0x0C);
        assert!((rpm - 2140.0).abs() < 0.01);
        assert_eq!(
            decode_freeze_frame_dtc(&[0x42, 0x02, 0x00, 0x01, 0x71]).unwrap(),
            Some("P0171".into())
        );
        assert_eq!(
            decode_freeze_frame_dtc(&[0x42, 0x02, 0x00, 0, 0]).unwrap(),
            None
        );
    }
}
