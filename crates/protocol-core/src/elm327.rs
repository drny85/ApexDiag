//! ELM327 / STN11xx command pipeline and response parser.
//!
//! An ELM327 is a text bridge: we type AT commands or hex requests ending in
//! `\r`, it talks to the car, and it prints the reply followed by a `>`
//! prompt. With headers on (`AT H1`) every received CAN frame comes back as a
//! line like `7E8 04 41 0C 1A F8`, which this module turns into [`CanFrame`]s.
//!
//! Cheap clones (including many BLE adapters) skip some commands or answer
//! `?` to them. The parser treats every adapter error as a typed value so
//! the UI can explain it instead of hanging.

use crate::bmw_e70::{Module, TESTER_ADDRESS, TESTER_ID};
use crate::CanId;

/// The init sequence sent after the link opens, in order.
pub const INIT_SEQUENCE: &[&str] = &[
    "AT Z",    // reset; replies with the version banner
    "AT E0",   // echo off
    "AT L0",   // linefeeds off
    "AT S1",   // spaces on; easier to read in logs, parser handles both
    "AT H1",   // headers on: we need the CAN ID of each reply
    "AT AT 1", // adaptive timing: faster replies on healthy buses
    "AT SP 6", // ISO 15765-4, 11-bit IDs, 500 kbaud (all OBD-II cars since 2008, and E70 D-CAN)
];

/// Format a command with the trailing carriage return the adapter expects.
pub fn command(cmd: &str) -> String {
    format!("{cmd}\r")
}

/// Hex-encodes a request the way the ELM327 expects it (`"22 F1 90"`).
pub fn hex_request(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Commands that point the adapter at a BMW E-series module on D-CAN.
///
/// * `AT SH 6F1`: send from the tester ID.
/// * `AT CEA xx`: insert the module address as the first data byte
///   (extended addressing). Needs ELM327 v1.4 or later.
/// * `AT CRA 6xx`: only listen to that module's replies.
/// * `AT FC ...`: our own Flow Control frame, addressed to the module, so
///   long replies (fault memory, identification) are not cut off.
pub fn bmw_module_setup(module: &Module) -> Vec<String> {
    vec![
        format!("AT SH {TESTER_ID:03X}"),
        format!("AT CEA {:02X}", module.address),
        format!("AT CRA {:03X}", module.response_id()),
        "AT FC SH 6F1".to_string(),
        format!("AT FC SD {:02X} 30 00 00", module.address),
        "AT FC SM 1".to_string(),
    ]
}

/// Commands that undo [`bmw_module_setup`] and return to OBD-II broadcast.
pub fn obd_broadcast_setup() -> Vec<String> {
    vec![
        "AT CEA".to_string(), // turn extended addressing off
        "AT FC SM 0".to_string(),
        "AT CRA".to_string(), // accept all replies
        "AT SH 7DF".to_string(),
    ]
}

/// A CAN frame printed by the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanFrame {
    pub id: CanId,
    pub data: Vec<u8>,
}

/// Errors the adapter reports in text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    /// The module did not answer. Usually ignition off or the wrong module.
    NoData,
    /// The adapter could not talk on the bus at all.
    UnableToConnect,
    /// Bus wiring or protocol mismatch.
    CanError,
    /// The adapter did not understand the command (common on clones).
    UnknownCommand,
    /// Replies arrived faster than the adapter could print them.
    BufferFull,
    /// A new command interrupted the previous one.
    Stopped,
    /// Corrupt frames or failed checksums.
    DataError,
    /// Battery voltage dropped and the adapter reset itself.
    LowVoltageReset,
    /// Any other text we did not recognise.
    Other(String),
}

impl AdapterError {
    /// Plain-language explanation for the UI.
    pub fn explain(&self) -> &'static str {
        match self {
            Self::NoData => "The module did not reply. Check the ignition is on and the module exists on this car.",
            Self::UnableToConnect => "The adapter could not reach the car's network. Check it is firmly plugged in and the ignition is on.",
            Self::CanError => "The adapter saw errors on the CAN bus. This is usually a protocol mismatch or a loose connection.",
            Self::UnknownCommand => "The adapter does not support this command. Many low-cost ELM327 clones skip advanced commands.",
            Self::BufferFull => "The car replied faster than the adapter could pass data on. Try again; Bluetooth LE adapters hit this more often.",
            Self::Stopped => "The request was interrupted by a newer one.",
            Self::DataError => "The adapter received corrupt data. Try again; if it repeats, check the connection.",
            Self::LowVoltageReset => "Battery voltage dropped low enough that the adapter reset. Connect a charger before continuing.",
            Self::Other(_) => "The adapter replied with an unexpected message.",
        }
    }
}

/// One complete reply, everything printed before the `>` prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// `OK` to an AT command.
    Ok,
    /// CAN frames, in the order received.
    Frames(Vec<CanFrame>),
    /// Free text, e.g. the version banner from `AT Z` or `AT RV` voltage.
    Text(String),
    Error(AdapterError),
}

/// True when the adapter has finished replying (it prints `>` when ready).
pub fn is_prompt(buf: &str) -> bool {
    buf.trim_end().ends_with('>')
}

/// Parses everything the adapter printed up to (and including) the prompt.
///
/// `sent` is the command we wrote; if echo is still on (right after reset)
/// the echoed line is dropped.
pub fn parse_response(raw: &str, sent: Option<&str>) -> Response {
    let body = raw.trim_end().trim_end_matches('>');
    let mut lines: Vec<&str> = body
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter(|l| !l.eq_ignore_ascii_case("SEARCHING..."))
        .collect();
    if let (Some(sent), Some(first)) = (sent, lines.first()) {
        if normalise(first) == normalise(sent) {
            lines.remove(0);
        }
    }
    if lines.is_empty() {
        return Response::Error(AdapterError::NoData);
    }
    for line in &lines {
        if let Some(err) = classify_error(line) {
            return Response::Error(err);
        }
    }
    if lines.len() == 1 && lines[0].eq_ignore_ascii_case("OK") {
        return Response::Ok;
    }
    let frames: Option<Vec<CanFrame>> = lines.iter().map(|l| parse_frame_line(l)).collect();
    match frames {
        Some(frames) => Response::Frames(frames),
        None => Response::Text(lines.join("\n")),
    }
}

fn normalise(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn classify_error(line: &str) -> Option<AdapterError> {
    let upper = line.to_ascii_uppercase();
    let err = match upper.as_str() {
        "NO DATA" => AdapterError::NoData,
        "?" => AdapterError::UnknownCommand,
        "CAN ERROR" => AdapterError::CanError,
        "BUFFER FULL" => AdapterError::BufferFull,
        "STOPPED" => AdapterError::Stopped,
        "DATA ERROR" => AdapterError::DataError,
        "LV RESET" => AdapterError::LowVoltageReset,
        u if u.starts_with("UNABLE TO CONNECT") => AdapterError::UnableToConnect,
        u if u.starts_with("BUS INIT") && u.contains("ERROR") => AdapterError::UnableToConnect,
        u if u.starts_with("<RX ERROR") || u.starts_with("<DATA ERROR") => AdapterError::DataError,
        u if u == "BUS BUSY" || u == "BUS ERROR" || u == "FB ERROR" => {
            AdapterError::Other(line.to_string())
        }
        _ => return None,
    };
    Some(err)
}

/// Parses `7E8 04 41 0C 1A F8` or `7E804410C1AF8` (spaces off).
/// Returns `None` for anything that is not an 11-bit frame line.
pub fn parse_frame_line(line: &str) -> Option<CanFrame> {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() < 5
        || compact.len().is_multiple_of(2)
        || !compact.chars().all(|c| c.is_ascii_hexdigit())
    {
        return None;
    }
    let id = u32::from_str_radix(&compact[..3], 16).ok()?;
    let data = (3..compact.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&compact[i..i + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .ok()?;
    if data.is_empty() || data.len() > 8 {
        return None;
    }
    Some(CanFrame { id, data })
}

/// Groups frames by sender ID, keeping arrival order. A functional OBD-II
/// request can be answered by several modules at once.
pub fn group_by_sender(frames: &[CanFrame]) -> Vec<(CanId, Vec<Vec<u8>>)> {
    let mut groups: Vec<(CanId, Vec<Vec<u8>>)> = Vec::new();
    for f in frames {
        match groups.iter_mut().find(|(id, _)| *id == f.id) {
            Some((_, list)) => list.push(f.data.clone()),
            None => groups.push((f.id, vec![f.data.clone()])),
        }
    }
    groups
}

/// For BMW replies: drop the tester address byte (`F1`) that leads every
/// frame when extended addressing is on.
pub fn strip_tester_address(frames: &[Vec<u8>]) -> Vec<Vec<u8>> {
    frames
        .iter()
        .map(|f| match f.split_first() {
            Some((&TESTER_ADDRESS, rest)) => rest.to_vec(),
            _ => f.clone(),
        })
        .collect()
}

/// Parses the `AT RV` battery voltage reply, e.g. `12.6V`.
pub fn parse_voltage(text: &str) -> Option<f32> {
    text.trim().trim_end_matches(['V', 'v']).parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bmw_e70::DME;

    #[test]
    fn init_starts_with_reset() {
        assert_eq!(INIT_SEQUENCE[0], "AT Z");
        assert_eq!(command("AT E0"), "AT E0\r");
    }

    #[test]
    fn detects_prompt() {
        assert!(is_prompt("OK\r\r>"));
        assert!(!is_prompt("SEARCHING..."));
    }

    #[test]
    fn parses_ok_and_banner() {
        assert_eq!(parse_response("OK\r\r>", None), Response::Ok);
        assert_eq!(
            parse_response("AT Z\r\rELM327 v1.5\r\r>", Some("AT Z")),
            Response::Text("ELM327 v1.5".into())
        );
    }

    #[test]
    fn parses_single_frame_with_headers() {
        let r = parse_response("SEARCHING...\r7E8 04 41 0C 1A F8 \r\r>", Some("01 0C"));
        assert_eq!(
            r,
            Response::Frames(vec![CanFrame {
                id: 0x7E8,
                data: vec![0x04, 0x41, 0x0C, 0x1A, 0xF8]
            }])
        );
    }

    #[test]
    fn parses_spaces_off() {
        assert_eq!(
            parse_frame_line("7E804410C1AF8"),
            Some(CanFrame {
                id: 0x7E8,
                data: vec![0x04, 0x41, 0x0C, 0x1A, 0xF8]
            })
        );
    }

    #[test]
    fn groups_multiple_ecus() {
        let r = parse_response(
            "7E8 06 41 00 BE 3E B8 11\r7E9 06 41 00 98 18 80 01\r\r>",
            None,
        );
        let Response::Frames(frames) = r else {
            panic!()
        };
        let groups = group_by_sender(&frames);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, 0x7E8);
        assert_eq!(groups[1].0, 0x7E9);
    }

    #[test]
    fn classifies_errors() {
        assert_eq!(
            parse_response("NO DATA\r\r>", None),
            Response::Error(AdapterError::NoData)
        );
        assert_eq!(
            parse_response("?\r\r>", None),
            Response::Error(AdapterError::UnknownCommand)
        );
        assert_eq!(
            parse_response("SEARCHING...\rUNABLE TO CONNECT\r\r>", None),
            Response::Error(AdapterError::UnableToConnect)
        );
        assert_eq!(
            parse_response(">", None),
            Response::Error(AdapterError::NoData)
        );
    }

    #[test]
    fn bmw_setup_targets_module() {
        let cmds = bmw_module_setup(&DME);
        assert_eq!(cmds[0], "AT SH 6F1");
        assert_eq!(cmds[1], "AT CEA 12");
        assert_eq!(cmds[2], "AT CRA 612");
        assert_eq!(cmds[4], "AT FC SD 12 30 00 00");
    }

    #[test]
    fn strips_tester_address() {
        let frames = vec![vec![0xF1, 0x03, 0x58, 0x00, 0x00]];
        assert_eq!(
            strip_tester_address(&frames),
            vec![vec![0x03, 0x58, 0x00, 0x00]]
        );
    }

    #[test]
    fn hex_and_voltage() {
        assert_eq!(hex_request(&[0x22, 0xF1, 0x90]), "22 F1 90");
        assert_eq!(parse_voltage("12.6V"), Some(12.6));
    }
}
