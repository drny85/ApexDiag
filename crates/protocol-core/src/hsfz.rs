//! HSFZ: BMW's diagnostics-over-Ethernet framing (ENET cable, F and G
//! series, TCP port 6801).
//!
//! Every frame is a 6-byte header followed by the payload:
//!
//! ```text
//! [length: u32 BE, payload bytes only] [control word: u16 BE] [payload]
//! ```
//!
//! A diagnostic frame's payload is `[source address] [target address]
//! [KWP2000/UDS bytes]`. The gateway first echoes the request back as an
//! acknowledge frame, then sends the module's reply as a diagnostic frame.
//!
//! This is groundwork: the codec is tested here, but it has not been run
//! against a car yet.

use crate::ProtocolError;

/// TCP port for diagnostic traffic.
pub const TCP_PORT: u16 = 6801;
/// UDP port for vehicle discovery.
pub const DISCOVERY_PORT: u16 = 6811;
/// The tester's address on ENET.
pub const TESTER_ADDRESS: u8 = 0xF4;
/// Largest payload we accept, so a corrupt length can't exhaust memory.
pub const MAX_PAYLOAD: usize = 0x10_0000;

/// Control words.
pub mod control {
    /// A diagnostic request or response.
    pub const DIAGNOSTIC: u16 = 0x0001;
    /// The gateway's echo of a request it accepted.
    pub const ACK: u16 = 0x0002;
    /// Vehicle identification (VIN, MAC, diagnostic address).
    pub const VEHICLE_IDENT: u16 = 0x0011;
    /// Keep-alive.
    pub const ALIVE_CHECK: u16 = 0x0012;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub control: u16,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn diagnostic(source: u8, target: u8, data: &[u8]) -> Self {
        let mut payload = Vec::with_capacity(data.len() + 2);
        payload.extend([source, target]);
        payload.extend_from_slice(data);
        Self {
            control: control::DIAGNOSTIC,
            payload,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(6 + self.payload.len());
        out.extend((self.payload.len() as u32).to_be_bytes());
        out.extend(self.control.to_be_bytes());
        out.extend_from_slice(&self.payload);
        out
    }

    /// For diagnostic and acknowledge frames: (source, target, data).
    pub fn addressed(&self) -> Option<(u8, u8, &[u8])> {
        match self.payload.as_slice() {
            [s, t, data @ ..] => Some((*s, *t, data)),
            _ => None,
        }
    }
}

/// Splits a byte stream into frames. Feed it whatever the socket returns.
#[derive(Debug, Default)]
pub struct Decoder {
    buf: Vec<u8>,
}

impl Decoder {
    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// The next complete frame, if one has arrived.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, ProtocolError> {
        if self.buf.len() < 6 {
            return Ok(None);
        }
        let len = u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]) as usize;
        if len > MAX_PAYLOAD {
            return Err(ProtocolError::MalformedFrame("HSFZ length too large"));
        }
        if self.buf.len() < 6 + len {
            return Ok(None);
        }
        let control = u16::from_be_bytes([self.buf[4], self.buf[5]]);
        let payload = self.buf[6..6 + len].to_vec();
        self.buf.drain(..6 + len);
        Ok(Some(Frame { control, payload }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_dme_fault_memory_request() {
        let f = Frame::diagnostic(TESTER_ADDRESS, 0x12, &[0x19, 0x02, 0x0C]);
        assert_eq!(
            f.encode(),
            vec![0, 0, 0, 5, 0x00, 0x01, 0xF4, 0x12, 0x19, 0x02, 0x0C]
        );
    }

    #[test]
    fn decodes_split_and_joined_frames() {
        let ack = Frame {
            control: control::ACK,
            payload: vec![0xF4, 0x12, 0x22, 0xF1, 0x90],
        };
        let reply = Frame::diagnostic(0x12, 0xF4, &[0x62, 0xF1, 0x90, b'W']);
        let mut bytes = ack.encode();
        bytes.extend(reply.encode());
        let mut d = Decoder::default();
        d.push(&bytes[..4]);
        assert_eq!(d.next_frame().unwrap(), None);
        d.push(&bytes[4..]);
        assert_eq!(d.next_frame().unwrap(), Some(ack));
        let got = d.next_frame().unwrap().unwrap();
        assert_eq!(
            got.addressed(),
            Some((0x12, 0xF4, &[0x62, 0xF1, 0x90, b'W'][..]))
        );
        assert_eq!(d.next_frame().unwrap(), None);
    }

    #[test]
    fn rejects_absurd_lengths() {
        let mut d = Decoder::default();
        d.push(&[0xFF, 0xFF, 0xFF, 0xFF, 0, 1]);
        assert!(d.next_frame().is_err());
    }
}
