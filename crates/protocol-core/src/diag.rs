//! Manufacturer diagnostics: KWP2000 (ISO 14230-3, BMW E-series on D-CAN)
//! and UDS (ISO 14229, BMW F/G-series over DoIP).
//!
//! Both are request/response protocols: the first byte of a request is the
//! service ID (SID). A positive reply starts with `SID + 0x40`; a negative
//! reply is `7F SID NRC`, where the Negative Response Code says why.

use crate::ProtocolError;
use serde::Serialize;

/// Which diagnostic protocol a module speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Kwp2000,
    Uds,
}

/// Service IDs used by the app.
pub mod sid {
    pub const DIAGNOSTIC_SESSION_CONTROL: u8 = 0x10;
    pub const ECU_RESET: u8 = 0x11;
    pub const CLEAR_DIAGNOSTIC_INFORMATION: u8 = 0x14;
    /// KWP2000: ReadDiagnosticTroubleCodesByStatus.
    pub const KWP_READ_DTC_BY_STATUS: u8 = 0x18;
    /// UDS: ReadDTCInformation.
    pub const UDS_READ_DTC_INFORMATION: u8 = 0x19;
    /// KWP2000: ReadECUIdentification.
    pub const KWP_READ_ECU_IDENTIFICATION: u8 = 0x1A;
    /// KWP2000: ReadDataByLocalIdentifier.
    pub const KWP_READ_DATA_BY_LOCAL_ID: u8 = 0x21;
    pub const READ_DATA_BY_IDENTIFIER: u8 = 0x22;
    pub const SECURITY_ACCESS: u8 = 0x27;
    /// UDS: WriteDataByIdentifier.
    pub const WRITE_DATA_BY_IDENTIFIER: u8 = 0x2E;
    /// UDS: InputOutputControlByIdentifier (actuator tests).
    pub const UDS_IO_CONTROL: u8 = 0x2F;
    /// KWP2000: InputOutputControlByLocalIdentifier (actuator tests).
    pub const KWP_IO_CONTROL_BY_LOCAL_ID: u8 = 0x30;
    /// KWP2000 StartRoutineByLocalIdentifier / UDS RoutineControl.
    pub const ROUTINE_CONTROL: u8 = 0x31;
    pub const TESTER_PRESENT: u8 = 0x3E;
    pub const NEGATIVE_RESPONSE: u8 = 0x7F;
}

/// Negative Response Codes shared by KWP2000 and UDS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Nrc(pub u8);

impl Nrc {
    pub const RESPONSE_PENDING: Nrc = Nrc(0x78);

    /// Short name as in the standards.
    pub fn name(self) -> &'static str {
        self.info().0
    }

    /// What it means and what to do about it, in plain words.
    pub fn explain(self) -> &'static str {
        self.info().1
    }

    fn info(self) -> (&'static str, &'static str) {
        match self.0 {
            0x10 => ("generalReject", "The module refused the request without giving a reason."),
            0x11 => ("serviceNotSupported", "This module does not support that kind of request."),
            0x12 => ("subFunctionNotSupported", "The module supports the service but not this variant of it."),
            0x13 => ("incorrectMessageLengthOrInvalidFormat", "The request was the wrong length or shape. This points to a bug or an unsupported module version."),
            0x14 => ("responseTooLong", "The reply would be too long to send."),
            0x21 => ("busyRepeatRequest", "The module is busy. The app will retry."),
            0x22 => ("conditionsNotCorrect", "The car is not in the right state for this, for example the engine is running or the gear is wrong."),
            0x24 => ("requestSequenceError", "Steps were sent out of order, for example a key before a seed."),
            0x25 => ("noResponseFromSubnetComponent", "The module could not reach a part it depends on."),
            0x26 => ("failurePreventsExecutionOfRequestedAction", "A fault in the module stops it doing this. Read its fault memory first."),
            0x31 => ("requestOutOfRange", "The module does not know that identifier or the value is out of range."),
            0x33 => ("securityAccessDenied", "This needs security access to be unlocked first."),
            0x35 => ("invalidKey", "The security key was wrong."),
            0x36 => ("exceedNumberOfAttempts", "Too many wrong security keys. The module is locked out for a while."),
            0x37 => ("requiredTimeDelayNotExpired", "The lock-out delay after failed attempts has not finished. Wait and try again."),
            0x70 => ("uploadDownloadNotAccepted", "The module refused to start a data transfer."),
            0x71 => ("transferDataSuspended", "A data transfer was interrupted."),
            0x72 => ("generalProgrammingFailure", "Writing to the module's memory failed."),
            0x73 => ("wrongBlockSequenceCounter", "A data block arrived out of order during a transfer."),
            0x78 => ("responsePending", "The module accepted the request and is still working on it."),
            0x7E => ("subFunctionNotSupportedInActiveSession", "This needs a different diagnostic session."),
            0x7F => ("serviceNotSupportedInActiveSession", "This needs a different diagnostic session."),
            0x81 => ("rpmTooHigh", "Engine speed is too high for this."),
            0x82 => ("rpmTooLow", "Engine speed is too low for this."),
            0x83 => ("engineIsRunning", "Switch the engine off and leave the ignition on."),
            0x84 => ("engineIsNotRunning", "Start the engine for this."),
            0x85 => ("engineRunTimeTooLow", "The engine has not been running long enough."),
            0x86 => ("temperatureTooHigh", "A temperature is too high for this."),
            0x87 => ("temperatureTooLow", "A temperature is too low; let the engine warm up."),
            0x88 => ("vehicleSpeedTooHigh", "The car must be stopped."),
            0x89 => ("vehicleSpeedTooLow", "The car must be moving for this."),
            0x8A => ("throttlePedalTooHigh", "Release the accelerator."),
            0x8B => ("throttlePedalTooLow", "Press the accelerator as instructed."),
            0x8C => ("transmissionRangeNotInNeutral", "Put the transmission in neutral."),
            0x8D => ("transmissionRangeNotInGear", "Put the transmission in gear."),
            0x8F => ("brakeSwitchNotClosed", "Press the brake pedal."),
            0x90 => ("shifterLeverNotInPark", "Put the transmission in Park."),
            0x91 => ("torqueConverterClutchLocked", "The torque converter clutch is locked."),
            0x92 => ("voltageTooHigh", "System voltage is too high."),
            0x93 => ("voltageTooLow", "Battery voltage is too low. Connect a charger."),
            _ => ("unknown", "The module gave a reason code this app does not know yet."),
        }
    }
}

/// Result of decoding one reply from a module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply<'a> {
    /// Positive reply; the slice is everything after the response SID.
    Positive(&'a [u8]),
    /// `7F SID 78`: keep waiting, a real reply follows.
    Pending,
    Negative(Nrc),
}

/// Decodes a reply to a request with service ID `request_sid`.
pub fn parse_reply(request_sid: u8, reply: &[u8]) -> Result<Reply<'_>, ProtocolError> {
    match reply {
        [sid::NEGATIVE_RESPONSE, rejected, nrc, ..] => {
            if *rejected != request_sid {
                return Err(ProtocolError::MalformedFrame(
                    "negative reply to another request",
                ));
            }
            if Nrc(*nrc) == Nrc::RESPONSE_PENDING {
                Ok(Reply::Pending)
            } else {
                Ok(Reply::Negative(Nrc(*nrc)))
            }
        }
        [first, rest @ ..] if *first == request_sid.wrapping_add(0x40) => Ok(Reply::Positive(rest)),
        [] => Err(ProtocolError::MalformedFrame("empty reply")),
        _ => Err(ProtocolError::MalformedFrame(
            "reply does not match request",
        )),
    }
}

/// Converts a reply into data or a [`ProtocolError::NegativeResponse`].
pub fn expect_positive(request_sid: u8, reply: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    match parse_reply(request_sid, reply)? {
        Reply::Positive(data) => Ok(data.to_vec()),
        Reply::Pending => Err(ProtocolError::MalformedFrame("response still pending")),
        Reply::Negative(nrc) => Err(ProtocolError::NegativeResponse {
            service: request_sid,
            nrc: nrc.0,
        }),
    }
}

/// Request builders.
pub mod request {
    use super::{sid, Protocol};

    /// Keep-alive. `3E 00` (UDS) or `3E 01` (KWP2000), both ask for a reply
    /// so a dropped session is noticed straight away.
    pub fn tester_present(protocol: Protocol) -> Vec<u8> {
        match protocol {
            Protocol::Uds => vec![sid::TESTER_PRESENT, 0x00],
            Protocol::Kwp2000 => vec![sid::TESTER_PRESENT, 0x01],
        }
    }

    /// Read all stored fault codes.
    pub fn read_dtcs(protocol: Protocol) -> Vec<u8> {
        match protocol {
            // statusOfDTC request, all groups
            Protocol::Kwp2000 => vec![sid::KWP_READ_DTC_BY_STATUS, 0x02, 0xFF, 0xFF],
            // reportDTCByStatusMask, any status
            Protocol::Uds => vec![sid::UDS_READ_DTC_INFORMATION, 0x02, 0xFF],
        }
    }

    /// Clear all fault codes in one module.
    pub fn clear_dtcs(protocol: Protocol) -> Vec<u8> {
        match protocol {
            Protocol::Kwp2000 => vec![sid::CLEAR_DIAGNOSTIC_INFORMATION, 0xFF, 0xFF],
            Protocol::Uds => vec![sid::CLEAR_DIAGNOSTIC_INFORMATION, 0xFF, 0xFF, 0xFF],
        }
    }

    /// Read a data identifier (UDS) or common identifier (KWP2000).
    pub fn read_data_by_identifier(did: u16) -> Vec<u8> {
        let [hi, lo] = did.to_be_bytes();
        vec![sid::READ_DATA_BY_IDENTIFIER, hi, lo]
    }

    /// KWP2000 ReadECUIdentification, `1A 80` = full identification table.
    pub fn kwp_read_identification() -> Vec<u8> {
        vec![sid::KWP_READ_ECU_IDENTIFICATION, 0x80]
    }

    /// Ask the module for a security seed at `level` (odd numbers).
    pub fn security_seed(level: u8) -> Vec<u8> {
        vec![sid::SECURITY_ACCESS, level]
    }
}

/// A fault code read from a BMW module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleDtc {
    /// Hex code as BMW tools display it: 4 digits on KWP2000 (`2A82`),
    /// 6 digits on UDS (`CD840A`).
    pub code: String,
    /// Raw status byte.
    pub status: u8,
    /// Decoded status flags, where the meaning is standardised.
    pub flags: Option<UdsDtcStatus>,
}

/// ISO 14229 DTC status bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UdsDtcStatus {
    /// Failing right now.
    pub test_failed: bool,
    /// Failed at least once this drive.
    pub failed_this_cycle: bool,
    /// Seen but not yet confirmed.
    pub pending: bool,
    /// Confirmed and stored.
    pub confirmed: bool,
    /// Asks for a warning light.
    pub warning_indicator: bool,
}

impl UdsDtcStatus {
    pub fn from_byte(b: u8) -> Self {
        Self {
            test_failed: b & 0x01 != 0,
            failed_this_cycle: b & 0x02 != 0,
            pending: b & 0x04 != 0,
            confirmed: b & 0x08 != 0,
            warning_indicator: b & 0x80 != 0,
        }
    }
}

/// Parses the data of a positive fault-memory reply (after the response SID).
pub fn parse_module_dtcs(protocol: Protocol, data: &[u8]) -> Result<Vec<ModuleDtc>, ProtocolError> {
    match protocol {
        Protocol::Kwp2000 => {
            // [count] then [hi lo status] * count
            let (&count, rest) = data
                .split_first()
                .ok_or(ProtocolError::MalformedFrame("missing DTC count"))?;
            if rest.len() < count as usize * 3 {
                return Err(ProtocolError::MalformedFrame("fewer DTCs than announced"));
            }
            Ok(rest
                .chunks_exact(3)
                .take(count as usize)
                .map(|c| ModuleDtc {
                    code: format!("{:02X}{:02X}", c[0], c[1]),
                    status: c[2],
                    flags: None,
                })
                .collect())
        }
        Protocol::Uds => {
            // [subfunction] [availability mask] then [b1 b2 b3 status] *
            let rest = data
                .get(2..)
                .ok_or(ProtocolError::MalformedFrame("missing DTC header"))?;
            Ok(rest
                .chunks_exact(4)
                .map(|c| ModuleDtc {
                    code: format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2]),
                    status: c[3],
                    flags: Some(UdsDtcStatus::from_byte(c[3])),
                })
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_negative_and_pending() {
        assert_eq!(
            parse_reply(0x18, &[0x58, 0x00]).unwrap(),
            Reply::Positive(&[0x00])
        );
        assert_eq!(
            parse_reply(0x31, &[0x7F, 0x31, 0x78]).unwrap(),
            Reply::Pending
        );
        assert_eq!(
            parse_reply(0x31, &[0x7F, 0x31, 0x22]).unwrap(),
            Reply::Negative(Nrc(0x22))
        );
        assert!(parse_reply(0x31, &[0x7F, 0x22, 0x22]).is_err());
        assert!(parse_reply(0x22, &[0x51]).is_err());
    }

    #[test]
    fn expect_positive_maps_nrc() {
        assert_eq!(
            expect_positive(0x27, &[0x7F, 0x27, 0x35]),
            Err(ProtocolError::NegativeResponse {
                service: 0x27,
                nrc: 0x35
            })
        );
        assert_eq!(Nrc(0x35).name(), "invalidKey");
        assert!(Nrc(0x93).explain().contains("charger"));
    }

    #[test]
    fn kwp_fault_memory() {
        // 58 02 | 2A 82 24 | 29 F3 20
        let dtcs = parse_module_dtcs(
            Protocol::Kwp2000,
            &[0x02, 0x2A, 0x82, 0x24, 0x29, 0xF3, 0x20],
        )
        .unwrap();
        assert_eq!(dtcs.len(), 2);
        assert_eq!(dtcs[0].code, "2A82");
        assert_eq!(dtcs[1].code, "29F3");
        assert!(parse_module_dtcs(Protocol::Kwp2000, &[0x02, 0x2A]).is_err());
    }

    #[test]
    fn uds_fault_memory() {
        let dtcs = parse_module_dtcs(Protocol::Uds, &[0x02, 0xFF, 0xCD, 0x84, 0x0A, 0x09]).unwrap();
        assert_eq!(dtcs[0].code, "CD840A");
        let f = dtcs[0].flags.unwrap();
        assert!(f.test_failed && f.confirmed && !f.pending);
    }

    #[test]
    fn requests() {
        assert_eq!(
            request::read_dtcs(Protocol::Kwp2000),
            vec![0x18, 0x02, 0xFF, 0xFF]
        );
        assert_eq!(request::tester_present(Protocol::Uds), vec![0x3E, 0x00]);
        assert_eq!(
            request::read_data_by_identifier(0xF190),
            vec![0x22, 0xF1, 0x90]
        );
    }
}
