//! DoIP (ISO 13400-2): diagnostics over IP, used by newer cars of most makes
//! (BMW G series, recent Mercedes-Benz, VW group and others), TCP port 13400.
//!
//! Every message has an 8-byte generic header:
//!
//! ```text
//! [protocol version] [inverse version] [payload type: u16 BE] [length: u32 BE]
//! ```
//!
//! A session opens with a routing activation; after that, diagnostic
//! messages carry `[source address: u16] [target address: u16] [UDS bytes]`
//! and each one is acknowledged before the module's reply arrives.
//!
//! This is groundwork: the codec is tested here, but it has not been run
//! against a car yet.

use crate::ProtocolError;

pub const TCP_PORT: u16 = 13400;
/// ISO 13400-2:2012.
pub const VERSION_2012: u8 = 0x02;
/// A tester address from the range the standard reserves for external
/// test equipment (0x0E00 to 0x0FFF).
pub const TESTER_ADDRESS: u16 = 0x0E80;
pub const MAX_PAYLOAD: usize = 0x10_0000;

pub mod payload {
    pub const GENERIC_NACK: u16 = 0x0000;
    pub const VEHICLE_IDENT_REQUEST: u16 = 0x0001;
    pub const VEHICLE_ANNOUNCEMENT: u16 = 0x0004;
    pub const ROUTING_ACTIVATION_REQUEST: u16 = 0x0005;
    pub const ROUTING_ACTIVATION_RESPONSE: u16 = 0x0006;
    pub const ALIVE_CHECK_REQUEST: u16 = 0x0007;
    pub const ALIVE_CHECK_RESPONSE: u16 = 0x0008;
    pub const DIAGNOSTIC_MESSAGE: u16 = 0x8001;
    pub const DIAGNOSTIC_ACK: u16 = 0x8002;
    pub const DIAGNOSTIC_NACK: u16 = 0x8003;
}

/// Routing activation response code for "routing activated".
pub const ROUTING_SUCCESS: u8 = 0x10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub payload_type: u16,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.payload.len());
        out.extend([VERSION_2012, !VERSION_2012]);
        out.extend(self.payload_type.to_be_bytes());
        out.extend((self.payload.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.payload);
        out
    }

    /// Default activation (type 0x00) for `source`.
    pub fn routing_activation(source: u16) -> Self {
        let mut payload = source.to_be_bytes().to_vec();
        payload.push(0x00);
        payload.extend([0, 0, 0, 0]);
        Self {
            payload_type: payload::ROUTING_ACTIVATION_REQUEST,
            payload,
        }
    }

    pub fn diagnostic(source: u16, target: u16, data: &[u8]) -> Self {
        let mut payload = Vec::with_capacity(4 + data.len());
        payload.extend(source.to_be_bytes());
        payload.extend(target.to_be_bytes());
        payload.extend_from_slice(data);
        Self {
            payload_type: payload::DIAGNOSTIC_MESSAGE,
            payload,
        }
    }

    /// For diagnostic messages and their acks: (source, target, rest).
    pub fn addressed(&self) -> Option<(u16, u16, &[u8])> {
        match self.payload.as_slice() {
            [s0, s1, t0, t1, rest @ ..] => Some((
                u16::from_be_bytes([*s0, *s1]),
                u16::from_be_bytes([*t0, *t1]),
                rest,
            )),
            _ => None,
        }
    }

    /// The response code of a routing activation response.
    pub fn routing_code(&self) -> Option<u8> {
        (self.payload_type == payload::ROUTING_ACTIVATION_RESPONSE)
            .then(|| self.payload.get(4).copied())
            .flatten()
    }
}

/// What a routing activation response code means.
pub fn explain_routing_code(code: u8) -> &'static str {
    match code {
        0x00 => "the tester address is unknown to the car",
        0x01 => "all diagnostic sockets are in use",
        0x02 => "the tester address is already in use on another socket",
        0x03 => "this socket is already active with another tester address",
        0x04 => "authentication is missing",
        0x05 => "the car rejected the confirmation",
        0x06 => "the activation type is not supported",
        0x07 => "a secure (TLS) connection is required",
        0x10 => "routing activated",
        0x11 => "routing will be activated once confirmed",
        _ => "an unknown or maker-specific reason",
    }
}

#[derive(Debug, Default)]
pub struct Decoder {
    buf: Vec<u8>,
}

impl Decoder {
    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn next_message(&mut self) -> Result<Option<Message>, ProtocolError> {
        if self.buf.len() < 8 {
            return Ok(None);
        }
        if self.buf[0] != !self.buf[1] {
            return Err(ProtocolError::MalformedFrame(
                "DoIP version bytes don't match",
            ));
        }
        let payload_type = u16::from_be_bytes([self.buf[2], self.buf[3]]);
        let len = u32::from_be_bytes([self.buf[4], self.buf[5], self.buf[6], self.buf[7]]) as usize;
        if len > MAX_PAYLOAD {
            return Err(ProtocolError::MalformedFrame("DoIP length too large"));
        }
        if self.buf.len() < 8 + len {
            return Ok(None);
        }
        let payload = self.buf[8..8 + len].to_vec();
        self.buf.drain(..8 + len);
        Ok(Some(Message {
            payload_type,
            payload,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_routing_activation() {
        assert_eq!(
            Message::routing_activation(TESTER_ADDRESS).encode(),
            vec![0x02, 0xFD, 0x00, 0x05, 0, 0, 0, 7, 0x0E, 0x80, 0x00, 0, 0, 0, 0]
        );
    }

    #[test]
    fn round_trips_a_diagnostic_message() {
        let m = Message::diagnostic(TESTER_ADDRESS, 0x0010, &[0x22, 0xF1, 0x90]);
        let mut d = Decoder::default();
        let bytes = m.encode();
        d.push(&bytes[..9]);
        assert_eq!(d.next_message().unwrap(), None);
        d.push(&bytes[9..]);
        let got = d.next_message().unwrap().unwrap();
        assert_eq!(
            got.addressed(),
            Some((0x0E80, 0x0010, &[0x22, 0xF1, 0x90][..]))
        );
    }

    #[test]
    fn reads_routing_response_and_rejects_bad_headers() {
        let resp = Message {
            payload_type: payload::ROUTING_ACTIVATION_RESPONSE,
            payload: vec![0x0E, 0x80, 0x10, 0x01, ROUTING_SUCCESS, 0, 0, 0, 0],
        };
        assert_eq!(resp.routing_code(), Some(ROUTING_SUCCESS));
        let mut d = Decoder::default();
        d.push(&[0x02, 0x02, 0, 1, 0, 0, 0, 0]);
        assert!(d.next_message().is_err());
    }
}
