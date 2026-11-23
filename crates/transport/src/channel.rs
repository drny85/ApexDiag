//! Message-level diagnostic channels.
//!
//! An ELM327 is a text bridge, so [`crate::elm::ElmClient`] turns requests
//! into AT commands and CAN frames. Professional interfaces work one level
//! higher: an ENET cable (HSFZ) or a DoIP gateway takes a whole KWP2000/UDS
//! request for a module and returns its whole reply. [`DiagChannel`] is that
//! common shape, so module features can run over any of them.
//!
//! The HSFZ and DoIP channels here are groundwork. They are tested against
//! in-memory fake gateways, not yet against a car.

use crate::elm::{ClientError, ElmClient, Result};
use crate::{HardwareTransport, TransportError};
use async_trait::async_trait;
use protocol_core::diag::{self, Reply};
use protocol_core::vehicle::Module;
use protocol_core::{doip, hsfz, ProtocolError};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Sends one diagnostic request to one module and returns the positive
/// reply data after the response SID.
#[async_trait]
pub trait DiagChannel: Send {
    async fn request(&mut self, module: &Module, req: &[u8]) -> Result<Vec<u8>>;
    /// What kind of interface this is, for logs and reports.
    fn kind(&self) -> &'static str;
}

#[async_trait]
impl<T: HardwareTransport> DiagChannel for ElmClient<T> {
    async fn request(&mut self, module: &Module, req: &[u8]) -> Result<Vec<u8>> {
        self.module(module, req).await
    }
    fn kind(&self) -> &'static str {
        "ELM327"
    }
}

/// How long a module may keep answering "response pending" (NRC 78).
const PENDING_LIMIT: Duration = Duration::from_secs(10);
const REPLY_TIMEOUT: Duration = Duration::from_secs(3);

fn sid_of(req: &[u8]) -> Result<u8> {
    req.first()
        .copied()
        .ok_or_else(|| ProtocolError::MalformedFrame("empty request").into())
}

/// Reads from `io` into a decoder until `next` yields something.
async fn read_until<S, D, F, T>(
    io: &mut S,
    dec: &mut D,
    timeout: Duration,
    mut next: F,
) -> Result<T>
where
    S: AsyncRead + Unpin + Send,
    D: Send,
    F: FnMut(&mut D) -> std::result::Result<Option<T>, ProtocolError> + Send,
    T: Send,
    D: Push,
{
    let deadline = tokio::time::Instant::now() + timeout;
    let mut buf = [0u8; 4096];
    loop {
        if let Some(t) = next(dec)? {
            return Ok(t);
        }
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        let n = tokio::time::timeout(left, io.read(&mut buf))
            .await
            .map_err(|_| TransportError::Timeout(timeout))?
            .map_err(TransportError::Io)?;
        if n == 0 {
            return Err(TransportError::Other("the interface closed the connection".into()).into());
        }
        dec.push(&buf[..n]);
    }
}

trait Push {
    fn push(&mut self, bytes: &[u8]);
}
impl Push for hsfz::Decoder {
    fn push(&mut self, bytes: &[u8]) {
        hsfz::Decoder::push(self, bytes)
    }
}
impl Push for doip::Decoder {
    fn push(&mut self, bytes: &[u8]) {
        doip::Decoder::push(self, bytes)
    }
}

/// Turns one module reply into the channel result, or `None` to keep
/// waiting after "response pending".
fn finish(sid: u8, msg: &[u8]) -> Result<Option<Vec<u8>>> {
    match diag::parse_reply(sid, msg)? {
        Reply::Pending => Ok(None),
        Reply::Positive(data) => Ok(Some(data.to_vec())),
        Reply::Negative(nrc) => Err(ProtocolError::NegativeResponse {
            service: sid,
            nrc: nrc.0,
        }
        .into()),
    }
}

/// BMW ENET (F and G series): HSFZ framing over TCP port 6801.
pub struct HsfzChannel<S> {
    io: S,
    dec: hsfz::Decoder,
}

impl HsfzChannel<tokio::net::TcpStream> {
    /// Connects to the car's gateway, usually 169.254.x.x on a direct cable.
    pub async fn connect(host: &str) -> Result<Self> {
        let io = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::net::TcpStream::connect((host, hsfz::TCP_PORT)),
        )
        .await
        .map_err(|_| TransportError::Timeout(Duration::from_secs(5)))?
        .map_err(TransportError::Io)?;
        Ok(Self::new(io))
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> HsfzChannel<S> {
    pub fn new(io: S) -> Self {
        Self {
            io,
            dec: hsfz::Decoder::default(),
        }
    }
}

#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Send> DiagChannel for HsfzChannel<S> {
    async fn request(&mut self, module: &Module, req: &[u8]) -> Result<Vec<u8>> {
        let sid = sid_of(req)?;
        let frame = hsfz::Frame::diagnostic(hsfz::TESTER_ADDRESS, module.address, req);
        self.io
            .write_all(&frame.encode())
            .await
            .map_err(TransportError::Io)?;
        let started = tokio::time::Instant::now();
        loop {
            let f = read_until(&mut self.io, &mut self.dec, REPLY_TIMEOUT, |d| {
                d.next_frame()
            })
            .await?;
            if f.control != hsfz::control::DIAGNOSTIC {
                continue; // the gateway's echo, or a keep-alive
            }
            match f.addressed() {
                Some((src, hsfz::TESTER_ADDRESS, data)) if src == module.address => {
                    if let Some(r) = finish(sid, data)? {
                        return Ok(r);
                    }
                    if started.elapsed() > PENDING_LIMIT {
                        return Err(ClientError::NoReply(module.name.into()));
                    }
                }
                _ => continue,
            }
        }
    }
    fn kind(&self) -> &'static str {
        "ENET (HSFZ)"
    }
}

/// ISO 13400 DoIP over TCP port 13400. `target` maps a module to its DoIP
/// logical address, which differs by maker.
pub struct DoipChannel<S> {
    io: S,
    dec: doip::Decoder,
    activated: bool,
    target: fn(&Module) -> u16,
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> DoipChannel<S> {
    pub fn new(io: S, target: fn(&Module) -> u16) -> Self {
        Self {
            io,
            dec: doip::Decoder::default(),
            activated: false,
            target,
        }
    }

    async fn send(&mut self, m: &doip::Message) -> Result<()> {
        self.io
            .write_all(&m.encode())
            .await
            .map_err(TransportError::Io)?;
        Ok(())
    }

    async fn next(&mut self) -> Result<doip::Message> {
        read_until(&mut self.io, &mut self.dec, REPLY_TIMEOUT, |d| {
            d.next_message()
        })
        .await
    }

    /// Opens routing so diagnostic messages reach the modules.
    pub async fn activate(&mut self) -> Result<()> {
        self.send(&doip::Message::routing_activation(doip::TESTER_ADDRESS))
            .await?;
        loop {
            let m = self.next().await?;
            if let Some(code) = m.routing_code() {
                if code == doip::ROUTING_SUCCESS {
                    self.activated = true;
                    return Ok(());
                }
                return Err(TransportError::Other(format!(
                    "The car refused the diagnostic session: {}.",
                    doip::explain_routing_code(code)
                ))
                .into());
            }
        }
    }
}

#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Send> DiagChannel for DoipChannel<S> {
    async fn request(&mut self, module: &Module, req: &[u8]) -> Result<Vec<u8>> {
        if !self.activated {
            self.activate().await?;
        }
        let sid = sid_of(req)?;
        let target = (self.target)(module);
        self.send(&doip::Message::diagnostic(
            doip::TESTER_ADDRESS,
            target,
            req,
        ))
        .await?;
        let started = tokio::time::Instant::now();
        loop {
            let m = self.next().await?;
            match m.payload_type {
                doip::payload::DIAGNOSTIC_NACK => {
                    return Err(TransportError::Other(format!(
                        "The gateway refused the request for {}.",
                        module.name
                    ))
                    .into())
                }
                doip::payload::ALIVE_CHECK_REQUEST => {
                    self.send(&doip::Message {
                        payload_type: doip::payload::ALIVE_CHECK_RESPONSE,
                        payload: doip::TESTER_ADDRESS.to_be_bytes().to_vec(),
                    })
                    .await?;
                }
                doip::payload::DIAGNOSTIC_MESSAGE => match m.addressed() {
                    Some((src, doip::TESTER_ADDRESS, data)) if src == target => {
                        if let Some(r) = finish(sid, data)? {
                            return Ok(r);
                        }
                        if started.elapsed() > PENDING_LIMIT {
                            return Err(ClientError::NoReply(module.name.into()));
                        }
                    }
                    _ => {}
                },
                _ => {} // positive acks and anything else
            }
        }
    }
    fn kind(&self) -> &'static str {
        "DoIP"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_core::bmw_e70::DME;
    use tokio::io::duplex;

    /// A fake ENET gateway: echoes the request, sends "pending", then the
    /// DME's identification.
    #[tokio::test]
    async fn hsfz_channel_handles_ack_and_pending() {
        let (client, mut car) = duplex(4096);
        let gateway = tokio::spawn(async move {
            let mut dec = hsfz::Decoder::default();
            let mut buf = [0u8; 256];
            let req = loop {
                let n = car.read(&mut buf).await.unwrap();
                dec.push(&buf[..n]);
                if let Some(f) = dec.next_frame().unwrap() {
                    break f;
                }
            };
            assert_eq!(req.addressed(), Some((0xF4, 0x12, &[0x22, 0xF1, 0x90][..])));
            let ack = hsfz::Frame {
                control: hsfz::control::ACK,
                payload: req.payload.clone(),
            };
            let pending = hsfz::Frame::diagnostic(0x12, 0xF4, &[0x7F, 0x22, 0x78]);
            let reply = hsfz::Frame::diagnostic(0x12, 0xF4, &[0x62, 0xF1, 0x90, b'W', b'B', b'A']);
            for f in [ack, pending, reply] {
                car.write_all(&f.encode()).await.unwrap();
            }
        });
        let mut ch = HsfzChannel::new(client);
        let data = ch.request(&DME, &[0x22, 0xF1, 0x90]).await.unwrap();
        assert_eq!(data, vec![0xF1, 0x90, b'W', b'B', b'A']);
        gateway.await.unwrap();
    }

    #[tokio::test]
    async fn doip_channel_activates_then_reads() {
        let (client, mut car) = duplex(4096);
        let gateway = tokio::spawn(async move {
            let mut dec = doip::Decoder::default();
            let mut buf = [0u8; 256];
            let mut seen = vec![];
            while seen.len() < 2 {
                let n = car.read(&mut buf).await.unwrap();
                dec.push(&buf[..n]);
                while let Some(m) = dec.next_message().unwrap() {
                    match m.payload_type {
                        doip::payload::ROUTING_ACTIVATION_REQUEST => {
                            let resp = doip::Message {
                                payload_type: doip::payload::ROUTING_ACTIVATION_RESPONSE,
                                payload: vec![
                                    0x0E,
                                    0x80,
                                    0x10,
                                    0x01,
                                    doip::ROUTING_SUCCESS,
                                    0,
                                    0,
                                    0,
                                    0,
                                ],
                            };
                            car.write_all(&resp.encode()).await.unwrap();
                        }
                        doip::payload::DIAGNOSTIC_MESSAGE => {
                            let (_, tgt, data) = m.addressed().unwrap();
                            assert_eq!((tgt, data), (0x0012, &[0x19, 0x02, 0xFF][..]));
                            let ack = doip::Message {
                                payload_type: doip::payload::DIAGNOSTIC_ACK,
                                payload: vec![0x00, 0x12, 0x0E, 0x80, 0x00],
                            };
                            let reply =
                                doip::Message::diagnostic(0x0012, 0x0E80, &[0x59, 0x02, 0xFF]);
                            car.write_all(&ack.encode()).await.unwrap();
                            car.write_all(&reply.encode()).await.unwrap();
                        }
                        _ => {}
                    }
                    seen.push(m.payload_type);
                }
            }
        });
        let mut ch = DoipChannel::new(client, |m| u16::from(m.address));
        let data = ch.request(&DME, &[0x19, 0x02, 0xFF]).await.unwrap();
        assert_eq!(data, vec![0x02, 0xFF]);
        gateway.await.unwrap();
    }

    #[tokio::test]
    async fn doip_reports_a_refused_activation() {
        let (client, mut car) = duplex(4096);
        tokio::spawn(async move {
            let mut buf = [0u8; 64];
            let _ = car.read(&mut buf).await.unwrap();
            let resp = doip::Message {
                payload_type: doip::payload::ROUTING_ACTIVATION_RESPONSE,
                payload: vec![0x0E, 0x80, 0x10, 0x01, 0x07, 0, 0, 0, 0],
            };
            car.write_all(&resp.encode()).await.unwrap();
        });
        let mut ch = DoipChannel::new(client, |m| u16::from(m.address));
        let e = ch.activate().await.unwrap_err();
        assert!(e.to_string().contains("secure (TLS)"), "{e}");
    }
}
