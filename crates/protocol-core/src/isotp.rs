//! ISO 15765-2 (ISO-TP) segmentation and reassembly for classic CAN.
//!
//! A diagnostic message can be up to 4095 bytes, but a CAN frame carries at
//! most 8. ISO-TP splits a long message into:
//!
//! * **Single Frame** (`0x0L`): the whole message fits in one frame.
//! * **First Frame** (`0x1L LL`): the first chunk plus the total length.
//! * **Consecutive Frames** (`0x2N`): the remaining chunks, `N` is a 4-bit
//!   sequence number that starts at 1 and wraps 15 -> 0.
//! * **Flow Control** (`0x3S BS ST`): sent by the receiver after a First
//!   Frame to say "go ahead", how many frames per block, and the gap between
//!   frames.
//!
//! BMW E-series modules on D-CAN use *extended addressing*: the first byte of
//! every frame is the target address (`0x12` for the DME on requests, `0xF1`
//! for the tester on replies), which leaves one byte less for data.
//!
//! Everything here is pure: frames in, frames out, no clocks or I/O.

use crate::ProtocolError;
use std::time::Duration;

/// Largest payload ISO-TP can carry with a 12-bit First Frame length.
pub const MAX_PAYLOAD: usize = 4095;
/// Classic CAN frame size.
pub const CAN_FRAME_LEN: usize = 8;

/// Flow Control frame `30 00 00`: ContinueToSend, BlockSize 0, STmin 0 ms.
pub const FLOW_CONTROL_CTS: [u8; 3] = [0x30, 0x00, 0x00];

/// How frames are addressed on the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Addressing {
    /// The CAN ID alone identifies the target (OBD-II, UDS on F/G chassis).
    #[default]
    Normal,
    /// The first data byte is the target address (BMW E-series D-CAN).
    Extended(u8),
}

impl Addressing {
    /// Bytes taken by the address prefix.
    pub const fn prefix_len(self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Extended(_) => 1,
        }
    }

    /// Payload bytes that fit in a Single Frame.
    pub const fn single_frame_capacity(self) -> usize {
        CAN_FRAME_LEN - 1 - self.prefix_len()
    }

    /// Payload bytes carried by a First Frame.
    pub const fn first_frame_capacity(self) -> usize {
        CAN_FRAME_LEN - 2 - self.prefix_len()
    }

    /// Payload bytes carried by each Consecutive Frame.
    pub const fn consecutive_frame_capacity(self) -> usize {
        CAN_FRAME_LEN - 1 - self.prefix_len()
    }

    fn write_prefix(self, frame: &mut Vec<u8>) {
        if let Self::Extended(addr) = self {
            frame.push(addr);
        }
    }

    /// Removes the address prefix from a received frame. For extended
    /// addressing the prefix is returned so callers can check who replied.
    pub fn strip(self, frame: &[u8]) -> Result<(Option<u8>, &[u8]), ProtocolError> {
        match self {
            Self::Normal => Ok((None, frame)),
            Self::Extended(_) => {
                let (addr, rest) = frame
                    .split_first()
                    .ok_or(ProtocolError::MalformedFrame("missing address byte"))?;
                Ok((Some(*addr), rest))
            }
        }
    }
}

/// Protocol Control Information type, the high nibble of the PCI byte.
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

/// Flow Control status, the low nibble of a `0x3X` frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowStatus {
    ContinueToSend,
    Wait,
    Overflow,
}

/// A decoded Flow Control frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowControl {
    pub status: FlowStatus,
    /// Consecutive Frames to send before waiting for the next Flow Control.
    /// `0` means "send everything".
    pub block_size: u8,
    /// Minimum gap between Consecutive Frames.
    pub st_min: Duration,
}

impl FlowControl {
    /// ContinueToSend, no block limit, no gap. Equivalent to `30 00 00`.
    pub const CONTINUE: Self = Self {
        status: FlowStatus::ContinueToSend,
        block_size: 0,
        st_min: Duration::ZERO,
    };

    /// Parses a Flow Control frame (address prefix already removed).
    pub fn parse(frame: &[u8]) -> Result<Self, ProtocolError> {
        let [pci, bs, st, ..] = *frame else {
            return Err(ProtocolError::MalformedFrame(
                "flow control shorter than 3 bytes",
            ));
        };
        if FrameType::from_pci(pci)? != FrameType::FlowControl {
            return Err(ProtocolError::UnexpectedFrame);
        }
        let status = match pci & 0x0F {
            0 => FlowStatus::ContinueToSend,
            1 => FlowStatus::Wait,
            2 => FlowStatus::Overflow,
            _ => return Err(ProtocolError::MalformedFrame("reserved flow status")),
        };
        Ok(Self {
            status,
            block_size: bs,
            st_min: decode_st_min(st),
        })
    }

    /// Encodes this Flow Control as a frame with the given addressing.
    pub fn encode(&self, addressing: Addressing) -> Vec<u8> {
        let mut frame = Vec::with_capacity(4);
        addressing.write_prefix(&mut frame);
        let status = match self.status {
            FlowStatus::ContinueToSend => 0,
            FlowStatus::Wait => 1,
            FlowStatus::Overflow => 2,
        };
        frame.extend_from_slice(&[0x30 | status, self.block_size, encode_st_min(self.st_min)]);
        frame
    }
}

/// STmin byte to a duration. `00-7F` are milliseconds, `F1-F9` are 100-900
/// microseconds. Reserved values must be treated as the maximum (127 ms).
pub fn decode_st_min(byte: u8) -> Duration {
    match byte {
        0x00..=0x7F => Duration::from_millis(byte as u64),
        0xF1..=0xF9 => Duration::from_micros((byte - 0xF0) as u64 * 100),
        _ => Duration::from_millis(0x7F),
    }
}

/// Duration to the closest STmin byte that is not shorter than requested.
pub fn encode_st_min(d: Duration) -> u8 {
    let micros = d.as_micros();
    if micros == 0 {
        0
    } else if micros < 1000 {
        0xF0 + micros.div_ceil(100) as u8
    } else {
        micros.div_ceil(1000).min(0x7F) as u8
    }
}

/// Splits a payload into CAN frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segmented {
    /// Fits in one frame; send it and you're done.
    Single(Vec<u8>),
    /// Send `first`, wait for Flow Control, then hand it to a [`Sender`].
    Multi { first: Vec<u8>, sender: Sender },
}

/// Segments `payload` for transmission.
pub fn segment(payload: &[u8], addressing: Addressing) -> Result<Segmented, ProtocolError> {
    if payload.is_empty() || payload.len() > MAX_PAYLOAD {
        return Err(ProtocolError::PayloadTooLong(payload.len()));
    }
    if payload.len() <= addressing.single_frame_capacity() {
        let mut frame = Vec::with_capacity(CAN_FRAME_LEN);
        addressing.write_prefix(&mut frame);
        frame.push(payload.len() as u8);
        frame.extend_from_slice(payload);
        return Ok(Segmented::Single(frame));
    }
    let len = payload.len();
    let take = addressing.first_frame_capacity();
    let mut first = Vec::with_capacity(CAN_FRAME_LEN);
    addressing.write_prefix(&mut first);
    first.push(0x10 | ((len >> 8) as u8 & 0x0F));
    first.push(len as u8);
    first.extend_from_slice(&payload[..take]);
    Ok(Segmented::Multi {
        first,
        sender: Sender {
            addressing,
            remaining: payload[take..].to_vec(),
            offset: 0,
            next_sn: 1,
        },
    })
}

/// Produces Consecutive Frames after a First Frame has been sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sender {
    addressing: Addressing,
    remaining: Vec<u8>,
    offset: usize,
    next_sn: u8,
}

impl Sender {
    pub fn is_done(&self) -> bool {
        self.offset >= self.remaining.len()
    }

    /// Frames to send in response to a Flow Control, plus the gap to leave
    /// between them. Call again on each new Flow Control until [`is_done`].
    ///
    /// [`is_done`]: Sender::is_done
    pub fn on_flow_control(
        &mut self,
        fc: FlowControl,
    ) -> Result<(Vec<Vec<u8>>, Duration), ProtocolError> {
        match fc.status {
            FlowStatus::Wait => return Ok((Vec::new(), fc.st_min)),
            FlowStatus::Overflow => {
                return Err(ProtocolError::MalformedFrame("receiver reported overflow"))
            }
            FlowStatus::ContinueToSend => {}
        }
        let cap = self.addressing.consecutive_frame_capacity();
        let limit = if fc.block_size == 0 {
            usize::MAX
        } else {
            fc.block_size as usize
        };
        let mut frames = Vec::new();
        while !self.is_done() && frames.len() < limit {
            let end = (self.offset + cap).min(self.remaining.len());
            let mut frame = Vec::with_capacity(CAN_FRAME_LEN);
            self.addressing.write_prefix(&mut frame);
            frame.push(0x20 | self.next_sn);
            frame.extend_from_slice(&self.remaining[self.offset..end]);
            frames.push(frame);
            self.offset = end;
            self.next_sn = (self.next_sn + 1) & 0x0F;
        }
        Ok((frames, fc.st_min))
    }
}

/// What the [`Reassembler`] needs next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    /// A whole message arrived.
    Complete(Vec<u8>),
    /// A First Frame arrived: send Flow Control, then keep feeding frames.
    SendFlowControl,
    /// Waiting for more Consecutive Frames.
    InProgress,
}

/// Rebuilds a message from received frames.
#[derive(Debug, Clone, Default)]
pub struct Reassembler {
    addressing: Addressing,
    expected: usize,
    buffer: Vec<u8>,
    next_sn: u8,
    active: bool,
}

impl Reassembler {
    pub fn new(addressing: Addressing) -> Self {
        Self {
            addressing,
            ..Self::default()
        }
    }

    pub fn is_idle(&self) -> bool {
        !self.active
    }

    /// Drops any partial message.
    pub fn reset(&mut self) {
        self.active = false;
        self.buffer.clear();
        self.expected = 0;
    }

    /// Feeds one received frame. Padding after the payload is ignored.
    pub fn push(&mut self, frame: &[u8]) -> Result<Progress, ProtocolError> {
        let (_, frame) = self.addressing.strip(frame)?;
        let pci = *frame
            .first()
            .ok_or(ProtocolError::MalformedFrame("empty frame"))?;
        match FrameType::from_pci(pci)? {
            FrameType::Single => {
                // A new Single Frame aborts any message in progress.
                self.reset();
                let len = (pci & 0x0F) as usize;
                if len == 0 {
                    return Err(ProtocolError::MalformedFrame("single frame length 0"));
                }
                let data = frame.get(1..1 + len).ok_or(ProtocolError::MalformedFrame(
                    "single frame shorter than its length",
                ))?;
                Ok(Progress::Complete(data.to_vec()))
            }
            FrameType::First => {
                let len_lo = *frame
                    .get(1)
                    .ok_or(ProtocolError::MalformedFrame("first frame missing length"))?;
                let expected = (((pci & 0x0F) as usize) << 8) | len_lo as usize;
                if expected == 0 {
                    return Err(ProtocolError::MalformedFrame(
                        "32-bit first frame length is not supported on classic CAN",
                    ));
                }
                if expected <= self.addressing.single_frame_capacity() {
                    return Err(ProtocolError::MalformedFrame(
                        "first frame length fits in a single frame",
                    ));
                }
                self.reset();
                self.active = true;
                self.expected = expected;
                self.next_sn = 1;
                let data = &frame[2..];
                self.buffer
                    .extend_from_slice(&data[..data.len().min(expected)]);
                Ok(Progress::SendFlowControl)
            }
            FrameType::Consecutive => {
                if !self.active {
                    return Err(ProtocolError::UnexpectedFrame);
                }
                let sn = pci & 0x0F;
                if sn != self.next_sn {
                    self.reset();
                    return Err(ProtocolError::MalformedFrame(
                        "consecutive frame out of order",
                    ));
                }
                self.next_sn = (self.next_sn + 1) & 0x0F;
                let want = self.expected - self.buffer.len();
                let data = &frame[1..];
                self.buffer.extend_from_slice(&data[..data.len().min(want)]);
                if self.buffer.len() >= self.expected {
                    self.active = false;
                    Ok(Progress::Complete(std::mem::take(&mut self.buffer)))
                } else {
                    Ok(Progress::InProgress)
                }
            }
            FrameType::FlowControl => Err(ProtocolError::UnexpectedFrame),
        }
    }
}

/// Reassembles a complete list of frames in one go (what an ELM327 prints
/// after it has already handled Flow Control itself).
pub fn reassemble(frames: &[Vec<u8>], addressing: Addressing) -> Result<Vec<u8>, ProtocolError> {
    let mut r = Reassembler::new(addressing);
    for frame in frames {
        if let Progress::Complete(msg) = r.push(frame)? {
            return Ok(msg);
        }
    }
    Err(ProtocolError::MalformedFrame(
        "message ended before all frames arrived",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames_for(payload: &[u8], addressing: Addressing) -> Vec<Vec<u8>> {
        match segment(payload, addressing).unwrap() {
            Segmented::Single(f) => vec![f],
            Segmented::Multi { first, mut sender } => {
                let (mut rest, _) = sender.on_flow_control(FlowControl::CONTINUE).unwrap();
                assert!(sender.is_done());
                rest.insert(0, first);
                rest
            }
        }
    }

    #[test]
    fn single_frame_round_trip() {
        let frames = frames_for(&[0x01, 0x0C], Addressing::Normal);
        assert_eq!(frames, vec![vec![0x02, 0x01, 0x0C]]);
        assert_eq!(
            reassemble(&frames, Addressing::Normal).unwrap(),
            vec![0x01, 0x0C]
        );
    }

    #[test]
    fn single_frame_ignores_padding() {
        let frame = vec![0x02, 0x41, 0x0D, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA];
        assert_eq!(
            reassemble(&[frame], Addressing::Normal).unwrap(),
            vec![0x41, 0x0D]
        );
    }

    #[test]
    fn vin_response_matches_real_obd_traffic() {
        // What a car sends for Mode 09 PID 02 (VIN), as printed by an ELM327.
        let frames = vec![
            vec![0x10, 0x14, 0x49, 0x02, 0x01, b'S', b'I', b'M'],
            vec![0x21, b'U', b'L', b'A', b'T', b'E', b'D', b'0'],
            vec![0x22, b'A', b'P', b'E', b'X', b'0', b'0', b'1'],
        ];
        let msg = reassemble(&frames, Addressing::Normal).unwrap();
        assert_eq!(msg.len(), 0x14);
        assert_eq!(&msg[..3], &[0x49, 0x02, 0x01]);
        assert_eq!(&msg[3..], b"SIMULATED0APEX001");
    }

    #[test]
    fn multi_frame_round_trip_all_lengths() {
        for addressing in [Addressing::Normal, Addressing::Extended(0x12)] {
            for len in 1..=300 {
                let payload: Vec<u8> = (0..len).map(|i| (i * 7) as u8).collect();
                let frames = frames_for(&payload, addressing);
                assert!(frames.iter().all(|f| f.len() <= CAN_FRAME_LEN));
                assert_eq!(
                    reassemble(&frames, addressing).unwrap(),
                    payload,
                    "len {len}"
                );
            }
        }
    }

    #[test]
    fn max_payload_round_trip_wraps_sequence_numbers() {
        let payload: Vec<u8> = (0..MAX_PAYLOAD).map(|i| i as u8).collect();
        let frames = frames_for(&payload, Addressing::Normal);
        // 6 bytes in the FF, 7 per CF.
        assert_eq!(frames.len(), 1 + (MAX_PAYLOAD - 6).div_ceil(7));
        assert_eq!(frames[16][0], 0x20, "sequence number wraps 15 -> 0");
        assert_eq!(reassemble(&frames, Addressing::Normal).unwrap(), payload);
        assert!(segment(&vec![0; MAX_PAYLOAD + 1], Addressing::Normal).is_err());
    }

    #[test]
    fn extended_addressing_layout() {
        // BMW DME fault memory request: 6F1 [12 04 18 02 FF FF]
        let Segmented::Single(f) =
            segment(&[0x18, 0x02, 0xFF, 0xFF], Addressing::Extended(0x12)).unwrap()
        else {
            panic!("expected single frame");
        };
        assert_eq!(f, vec![0x12, 0x04, 0x18, 0x02, 0xFF, 0xFF]);

        // 7 bytes no longer fit in a single frame once the address takes one.
        let Segmented::Multi { first, .. } = segment(&[0; 7], Addressing::Extended(0x12)).unwrap()
        else {
            panic!("expected multi frame");
        };
        assert_eq!(&first[..3], &[0x12, 0x10, 0x07]);
    }

    #[test]
    fn block_size_limits_frames_per_flow_control() {
        let payload = vec![0xAB; 50];
        let Segmented::Multi { mut sender, .. } = segment(&payload, Addressing::Normal).unwrap()
        else {
            panic!()
        };
        let fc = FlowControl {
            status: FlowStatus::ContinueToSend,
            block_size: 2,
            st_min: Duration::from_millis(5),
        };
        let (frames, gap) = sender.on_flow_control(fc).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(gap, Duration::from_millis(5));
        assert!(!sender.is_done());
        let (frames, _) = sender
            .on_flow_control(FlowControl {
                status: FlowStatus::Wait,
                ..fc
            })
            .unwrap();
        assert!(frames.is_empty());
    }

    #[test]
    fn out_of_order_consecutive_frame_is_rejected() {
        let mut r = Reassembler::new(Addressing::Normal);
        assert_eq!(
            r.push(&[0x10, 0x0A, 1, 2, 3, 4, 5, 6]).unwrap(),
            Progress::SendFlowControl
        );
        assert!(r.push(&[0x22, 7, 8, 9, 10]).is_err());
        assert!(r.is_idle());
    }

    #[test]
    fn stray_consecutive_frame_is_rejected() {
        let mut r = Reassembler::new(Addressing::Normal);
        assert_eq!(r.push(&[0x21, 1, 2]), Err(ProtocolError::UnexpectedFrame));
    }

    #[test]
    fn flow_control_round_trip() {
        assert_eq!(
            FlowControl::CONTINUE.encode(Addressing::Normal),
            FLOW_CONTROL_CTS.to_vec()
        );
        assert_eq!(
            FlowControl::parse(&FLOW_CONTROL_CTS).unwrap(),
            FlowControl::CONTINUE
        );
        let bmw = FlowControl::CONTINUE.encode(Addressing::Extended(0x12));
        assert_eq!(bmw, vec![0x12, 0x30, 0x00, 0x00]);
    }

    #[test]
    fn st_min_encoding() {
        assert_eq!(decode_st_min(0x0A), Duration::from_millis(10));
        assert_eq!(decode_st_min(0xF3), Duration::from_micros(300));
        assert_eq!(decode_st_min(0x80), Duration::from_millis(127));
        assert_eq!(encode_st_min(Duration::from_micros(300)), 0xF3);
        assert_eq!(encode_st_min(Duration::from_millis(20)), 20);
        assert_eq!(encode_st_min(Duration::from_micros(1500)), 2);
        assert_eq!(encode_st_min(Duration::from_secs(5)), 0x7F);
    }

    #[test]
    fn pci_types() {
        assert_eq!(FrameType::from_pci(0x10).unwrap(), FrameType::First);
        assert_eq!(FrameType::from_pci(0x21).unwrap(), FrameType::Consecutive);
        assert!(FrameType::from_pci(0x40).is_err());
    }
}
