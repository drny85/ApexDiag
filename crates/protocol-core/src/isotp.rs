//! ISO 15765-2 (ISO-TP) segmentation and reassembly.
//!
//! Step 2 of the plan fills in the multi-frame encoder/decoder. For now this
//! defines the frame model and handles Single Frames so the rest of the stack
//! has something real to call.

use crate::ProtocolError;

/// Largest payload that fits in a classic-CAN Single Frame.
pub const SF_MAX: usize = 7;
/// Largest payload ISO-TP can carry with a 12-bit First Frame length.
pub const MAX_PAYLOAD: usize = 4095;

/// Protocol Control Information type, the high nibble of the first byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Single,
    First,
    Consecutive,
    FlowControl,
}

impl FrameType {
    pub fn from_pci(byte: u8) -> Result<Self, ProtocolError> {
        match byte >> 4 {
            0x0 => Ok(Self::Single),
            0x1 => Ok(Self::First),
            0x2 => Ok(Self::Consecutive),
            0x3 => Ok(Self::FlowControl),
            _ => Err(ProtocolError::MalformedFrame("unknown PCI type")),
        }
    }
}

/// Flow Control frame `30 00 00`: ContinueToSend, BlockSize 0, STmin 0 ms.
pub const FLOW_CONTROL_CTS: [u8; 3] = [0x30, 0x00, 0x00];

/// Encode a payload of up to 7 bytes as a Single Frame (unpadded).
pub fn encode_single(payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if payload.is_empty() || payload.len() > SF_MAX {
        return Err(ProtocolError::PayloadTooLong(payload.len()));
    }
    let mut frame = Vec::with_capacity(payload.len() + 1);
    frame.push(payload.len() as u8);
    frame.extend_from_slice(payload);
    Ok(frame)
}

/// Decode a Single Frame back into its payload.
pub fn decode_single(frame: &[u8]) -> Result<&[u8], ProtocolError> {
    let pci = *frame
        .first()
        .ok_or(ProtocolError::MalformedFrame("empty frame"))?;
    if FrameType::from_pci(pci)? != FrameType::Single {
        return Err(ProtocolError::UnexpectedFrame);
    }
    let len = (pci & 0x0F) as usize;
    frame.get(1..1 + len).ok_or(ProtocolError::MalformedFrame(
        "single frame shorter than its length",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_frame_round_trip() {
        let frame = encode_single(&[0x01, 0x0C]).unwrap();
        assert_eq!(frame, vec![0x02, 0x01, 0x0C]);
        assert_eq!(decode_single(&frame).unwrap(), &[0x01, 0x0C]);
    }

    #[test]
    fn single_frame_rejects_eight_bytes() {
        assert!(encode_single(&[0; 8]).is_err());
    }

    #[test]
    fn pci_types() {
        assert_eq!(FrameType::from_pci(0x10).unwrap(), FrameType::First);
        assert_eq!(FrameType::from_pci(0x21).unwrap(), FrameType::Consecutive);
        assert_eq!(
            FrameType::from_pci(FLOW_CONTROL_CTS[0]).unwrap(),
            FrameType::FlowControl
        );
    }
}
