//! High-level client for ELM327-compatible adapters.
//!
//! Wraps any [`HardwareTransport`] and speaks the adapter's text protocol:
//! initialisation, OBD-II requests to all engine modules, and BMW requests
//! to a single module with extended addressing.

use crate::{HardwareTransport, TransportError};
use protocol_core::bmw_e70::{self, Module};
use protocol_core::diag::{self, request, ModuleDtc, Protocol, Reply};
use protocol_core::elm327::{self, AdapterError, Response};
use protocol_core::isotp::{Addressing, Progress, Reassembler};
use protocol_core::obd2::{self, Dtc, Service};
use protocol_core::{CanId, ProtocolError};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("adapter: {}", .0.explain())]
    Adapter(AdapterError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("the adapter does not support `{0}`")]
    Unsupported(String),
    #[error("no reply from {0}")]
    NoReply(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;

/// What the adapter said about itself during init.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AdapterInfo {
    pub version: String,
    /// Whether `AT CEA` (needed for BMW modules) is accepted.
    pub supports_extended_addressing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Unknown,
    Obd,
    Bmw(u8),
}

pub struct ElmClient<T: HardwareTransport> {
    link: T,
    timeout: Duration,
    target: Target,
    info: Option<AdapterInfo>,
}

impl<T: HardwareTransport> ElmClient<T> {
    pub fn new(link: T) -> Self {
        Self {
            link,
            timeout: Duration::from_secs(3),
            target: Target::Unknown,
            info: None,
        }
    }

    pub fn info(&self) -> Option<&AdapterInfo> {
        self.info.as_ref()
    }

    pub fn link_mut(&mut self) -> &mut T {
        &mut self.link
    }

    /// Opens the link and runs the init sequence.
    pub async fn connect(&mut self) -> Result<AdapterInfo> {
        if !self.link.is_connected() {
            self.link.connect().await?;
        }
        let mut version = String::from("unknown");
        for cmd in elm327::INIT_SEQUENCE {
            match self.exchange(cmd).await? {
                Response::Text(t) if *cmd == "AT Z" => version = t,
                Response::Ok | Response::Text(_) => {}
                Response::Error(AdapterError::UnknownCommand) if *cmd == "AT AT 1" => {}
                Response::Error(AdapterError::UnknownCommand) => {
                    return Err(ClientError::Unsupported(cmd.to_string()))
                }
                Response::Error(e) => return Err(ClientError::Adapter(e)),
                Response::Frames(_) => {}
            }
        }
        // Probe extended addressing once, then switch it straight back off.
        let supports_extended_addressing =
            matches!(self.exchange("AT CEA 12").await?, Response::Ok);
        if supports_extended_addressing {
            self.at("AT CEA").await?;
        }
        self.target = Target::Unknown;
        let info = AdapterInfo {
            version,
            supports_extended_addressing,
        };
        self.info = Some(info.clone());
        Ok(info)
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        self.link.disconnect().await?;
        self.target = Target::Unknown;
        Ok(())
    }

    /// Sends one line and reads everything up to the `>` prompt.
    pub async fn exchange(&mut self, line: &str) -> Result<Response> {
        self.link.send(elm327::command(line).as_bytes()).await?;
        let mut buf = String::new();
        let deadline = tokio::time::Instant::now() + self.timeout;
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                return Err(TransportError::Timeout(self.timeout).into());
            }
            match self.link.recv(left.min(Duration::from_millis(250))).await {
                Ok(bytes) => {
                    buf.push_str(&String::from_utf8_lossy(&bytes));
                    if elm327::is_prompt(&buf) {
                        return Ok(elm327::parse_response(&buf, Some(line)));
                    }
                }
                Err(TransportError::Timeout(_)) => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Sends an AT command that must answer `OK`.
    pub async fn at(&mut self, cmd: &str) -> Result<()> {
        match self.exchange(cmd).await? {
            Response::Ok | Response::Text(_) => Ok(()),
            Response::Error(AdapterError::UnknownCommand) => {
                Err(ClientError::Unsupported(cmd.into()))
            }
            Response::Error(e) => Err(ClientError::Adapter(e)),
            Response::Frames(_) => Ok(()),
        }
    }

    /// Battery voltage measured at the OBD port.
    pub async fn battery_voltage(&mut self) -> Result<f32> {
        match self.exchange("AT RV").await? {
            Response::Text(t) => elm327::parse_voltage(&t).ok_or(ClientError::Protocol(
                ProtocolError::MalformedFrame("bad voltage reply"),
            )),
            Response::Error(e) => Err(ClientError::Adapter(e)),
            _ => Err(ClientError::Protocol(ProtocolError::MalformedFrame(
                "bad voltage reply",
            ))),
        }
    }

    // ---- OBD-II -------------------------------------------------------

    async fn select_obd(&mut self) -> Result<()> {
        if self.target != Target::Obd {
            for cmd in elm327::obd_broadcast_setup() {
                // Older adapters may not know CEA/FC; that is fine for OBD.
                match self.at(&cmd).await {
                    Ok(()) | Err(ClientError::Unsupported(_)) => {}
                    Err(e) => return Err(e),
                }
            }
            self.target = Target::Obd;
        }
        Ok(())
    }

    /// Sends an OBD-II request and returns each module's reassembled reply.
    pub async fn obd(&mut self, req: &[u8]) -> Result<Vec<(CanId, Vec<u8>)>> {
        self.select_obd().await?;
        let frames = match self.exchange(&elm327::hex_request(req)).await? {
            Response::Frames(f) => f,
            Response::Error(e) => return Err(ClientError::Adapter(e)),
            _ => return Err(ProtocolError::MalformedFrame("expected CAN frames").into()),
        };
        let mut out = Vec::new();
        for (id, frames) in elm327::group_by_sender(&frames) {
            for msg in messages(&frames, Addressing::Normal)? {
                out.push((id, msg));
            }
        }
        Ok(out)
    }

    /// The engine module's reply (0x7E8) or the first one that arrived.
    async fn obd_primary(&mut self, req: &[u8]) -> Result<Vec<u8>> {
        let mut replies = self.obd(req).await?;
        let idx = replies.iter().position(|(id, _)| *id == 0x7E8).unwrap_or(0);
        if replies.is_empty() {
            return Err(ClientError::NoReply("engine module".into()));
        }
        Ok(replies.swap_remove(idx).1)
    }

    /// Reads one live value.
    pub async fn read_pid(&mut self, pid: u8) -> Result<f64> {
        let payload = self.obd_primary(&obd2::current_data_request(pid)).await?;
        Ok(obd2::decode_current_data(&payload)?.1)
    }

    /// All Service 01 PIDs the engine module supports.
    pub async fn supported_pids(&mut self) -> Result<Vec<u8>> {
        let mut all = Vec::new();
        let mut base = 0x00u8;
        loop {
            let payload = self.obd_primary(&[0x01, base]).await?;
            let pids = obd2::decode_supported_pids(&payload)?;
            let more = pids.contains(&(base.wrapping_add(0x20)));
            all.extend(pids.into_iter().filter(|p| p % 0x20 != 0));
            if !more || base >= 0xC0 {
                break;
            }
            base += 0x20;
        }
        Ok(all)
    }

    pub async fn read_vin(&mut self) -> Result<String> {
        let payload = self.obd_primary(&obd2::VIN_REQUEST).await?;
        Ok(obd2::decode_vin(&payload)?)
    }

    /// Generic emissions codes for one of Service 03 / 07 / 0A.
    pub async fn read_obd_dtcs(&mut self, service: Service) -> Result<Vec<Dtc>> {
        let payload = self.obd_primary(&[service as u8]).await?;
        Ok(obd2::decode_dtcs(service, &payload)?)
    }

    /// Clears emissions codes. Callers must check the safety gates first.
    pub async fn clear_obd_dtcs(&mut self) -> Result<()> {
        let payload = self.obd_primary(&[Service::ClearDtcs as u8]).await?;
        match payload.first() {
            Some(0x44) => Ok(()),
            _ => Err(ProtocolError::UnexpectedFrame.into()),
        }
    }

    // ---- BMW ----------------------------------------------------------

    async fn select_bmw(&mut self, module: &Module) -> Result<()> {
        if self.target != Target::Bmw(module.address) {
            if self
                .info
                .as_ref()
                .is_some_and(|i| !i.supports_extended_addressing)
            {
                return Err(ClientError::Unsupported(
                    "AT CEA (BMW extended addressing)".into(),
                ));
            }
            for cmd in elm327::bmw_module_setup(module) {
                self.at(&cmd).await?;
            }
            self.target = Target::Bmw(module.address);
        }
        Ok(())
    }

    /// Sends a KWP2000 request to one module and returns the positive reply
    /// data (after the response SID).
    pub async fn bmw(&mut self, module: &Module, req: &[u8]) -> Result<Vec<u8>> {
        self.select_bmw(module).await?;
        let sid = *req
            .first()
            .ok_or(ProtocolError::MalformedFrame("empty request"))?;
        let frames = match self.exchange(&elm327::hex_request(req)).await? {
            Response::Frames(f) => f,
            Response::Error(AdapterError::NoData) => {
                return Err(ClientError::NoReply(module.name.into()))
            }
            Response::Error(e) => return Err(ClientError::Adapter(e)),
            _ => return Err(ProtocolError::MalformedFrame("expected CAN frames").into()),
        };
        let mine: Vec<Vec<u8>> = frames
            .into_iter()
            .filter(|f| f.id == module.response_id())
            .map(|f| f.data)
            .collect();
        for msg in messages(&mine, Addressing::Extended(bmw_e70::TESTER_ADDRESS))? {
            match diag::parse_reply(sid, &msg)? {
                Reply::Pending => continue,
                Reply::Positive(data) => return Ok(data.to_vec()),
                Reply::Negative(nrc) => {
                    return Err(ProtocolError::NegativeResponse {
                        service: sid,
                        nrc: nrc.0,
                    }
                    .into())
                }
            }
        }
        Err(ClientError::NoReply(module.name.into()))
    }

    /// Whether a module answers at all.
    pub async fn probe(&mut self, module: &Module) -> Result<bool> {
        match self.bmw(module, &bmw_e70::scan_request()).await {
            Ok(_) => Ok(true),
            // A negative reply still proves the module is there.
            Err(ClientError::Protocol(ProtocolError::NegativeResponse { .. })) => Ok(true),
            Err(ClientError::NoReply(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Reads a module's fault memory.
    pub async fn read_module_dtcs(&mut self, module: &Module) -> Result<Vec<ModuleDtc>> {
        let data = self
            .bmw(module, &request::read_dtcs(Protocol::Kwp2000))
            .await?;
        Ok(diag::parse_module_dtcs(Protocol::Kwp2000, &data)?)
    }

    /// Reads a module's identification block as printable text.
    /// Reads one Time Machine record (`21 lid`).
    pub async fn read_record(&mut self, module: &Module, lid: u8) -> Result<Vec<u8>> {
        let data = self.bmw(module, &request::kwp_read_local_id(lid)).await?;
        match data.split_first() {
            Some((&echo, value)) if echo == lid => Ok(value.to_vec()),
            _ => Err(ProtocolError::MalformedFrame("record reply for another id").into()),
        }
    }

    /// Writes one coding record (`3B lid data`).
    pub async fn write_record(&mut self, module: &Module, lid: u8, value: &[u8]) -> Result<()> {
        let data = self
            .bmw(module, &request::kwp_write_local_id(lid, value))
            .await?;
        match data.first() {
            Some(&echo) if echo == lid => Ok(()),
            _ => Err(ProtocolError::MalformedFrame("write reply for another id").into()),
        }
    }

    pub async fn read_module_identification(&mut self, module: &Module) -> Result<String> {
        let data = self
            .bmw(module, &request::kwp_read_identification())
            .await?;
        let text: String = data
            .iter()
            .skip(1)
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        Ok(text.trim().to_string())
    }
}

/// Splits a sender's frames into complete messages, in arrival order.
fn messages(frames: &[Vec<u8>], addressing: Addressing) -> Result<Vec<Vec<u8>>> {
    let mut r = Reassembler::new(addressing);
    let mut out = Vec::new();
    for f in frames {
        if let Progress::Complete(m) = r.push(f)? {
            out.push(m);
        }
    }
    if !r.is_idle() {
        return Err(ProtocolError::MalformedFrame("reply ended mid-message").into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{ElmSimulator, SimEngine, SIM_VIN};

    async fn client(engine: SimEngine) -> ElmClient<ElmSimulator> {
        let mut c = ElmClient::new(ElmSimulator::new(engine));
        let info = c.connect().await.unwrap();
        assert_eq!(info.version, "ELM327 v1.5");
        assert!(info.supports_extended_addressing);
        c
    }

    #[tokio::test]
    async fn reads_live_data() {
        let mut c = client(SimEngine::Idling).await;
        let rpm = c.read_pid(0x0C).await.unwrap();
        assert!((650.0..800.0).contains(&rpm), "rpm {rpm}");
        assert_eq!(c.read_pid(0x0D).await.unwrap(), 0.0);
        let volts = c.read_pid(0x42).await.unwrap();
        assert!(volts > 13.5);
        assert!(c.battery_voltage().await.unwrap() > 13.0);
    }

    #[tokio::test]
    async fn engine_off_reads_zero_rpm() {
        let mut c = client(SimEngine::IgnitionOn).await;
        assert_eq!(c.read_pid(0x0C).await.unwrap(), 0.0);
    }

    #[tokio::test]
    async fn reads_multi_frame_vin() {
        let mut c = client(SimEngine::Idling).await;
        assert_eq!(c.read_vin().await.unwrap(), SIM_VIN);
    }

    #[tokio::test]
    async fn supported_pids_follow_the_chain() {
        let mut c = client(SimEngine::Idling).await;
        let pids = c.supported_pids().await.unwrap();
        assert!(pids.contains(&0x0C) && pids.contains(&0x42));
        assert!(pids.iter().all(|p| p % 0x20 != 0));
    }

    #[tokio::test]
    async fn functional_request_hears_two_modules() {
        let mut c = client(SimEngine::Idling).await;
        let replies = c.obd(&[0x01, 0x00]).await.unwrap();
        let ids: Vec<_> = replies.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![0x7E8, 0x7E9]);
    }

    #[tokio::test]
    async fn reads_and_clears_obd_codes() {
        let mut c = client(SimEngine::Idling).await;
        let codes = c.read_obd_dtcs(Service::StoredDtcs).await.unwrap();
        let names: Vec<_> = codes.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(names, vec!["P0171", "P0174"]);
        assert_eq!(
            c.read_obd_dtcs(Service::PendingDtcs).await.unwrap()[0].code,
            "P0300"
        );
        c.clear_obd_dtcs().await.unwrap();
        assert!(c
            .read_obd_dtcs(Service::StoredDtcs)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn reads_bmw_module_fault_memory_over_extended_addressing() {
        let mut c = client(SimEngine::IgnitionOn).await;
        let dtcs = c.read_module_dtcs(&bmw_e70::DME).await.unwrap();
        let codes: Vec<_> = dtcs.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(codes, vec!["29F3", "2A82"]);
        // The client set up extended addressing for the DME.
        let log = &c.link_mut().log;
        assert!(log.iter().any(|l| l == "AT CEA 12"));
        assert!(log.iter().any(|l| l == "AT CRA 612"));
    }

    #[tokio::test]
    async fn switches_between_obd_and_bmw() {
        let mut c = client(SimEngine::Idling).await;
        assert!(!c.read_module_dtcs(&bmw_e70::DSC).await.unwrap().is_empty());
        // Back to broadcast OBD without reconnecting.
        assert!(c.read_pid(0x0C).await.unwrap() > 0.0);
        let id = c.read_module_identification(&bmw_e70::EGS).await.unwrap();
        assert!(id.contains("EGS"), "{id}");
    }

    #[tokio::test]
    async fn module_scan_finds_fitted_modules_only() {
        let mut c = client(SimEngine::IgnitionOn).await;
        assert!(c.probe(&bmw_e70::DME).await.unwrap());
        assert!(!c.probe(&bmw_e70::PDC).await.unwrap());
    }

    #[tokio::test]
    async fn writes_are_refused_with_an_explained_reason() {
        let mut c = client(SimEngine::IgnitionOn).await;
        let err = c.bmw(&bmw_e70::DME, &[0x31, 0x01, 0x02]).await.unwrap_err();
        match err {
            ClientError::Protocol(ProtocolError::NegativeResponse { service: 0x31, nrc }) => {
                assert_eq!(diag::Nrc(nrc).name(), "conditionsNotCorrect");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    fn nrc_of(err: ClientError) -> u8 {
        match err {
            ClientError::Protocol(ProtocolError::NegativeResponse { nrc, .. }) => nrc,
            other => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn reads_and_writes_coding_records() {
        let mut c = client(SimEngine::IgnitionOn).await;
        // Daytime running lights, FRM.
        assert_eq!(c.read_record(&bmw_e70::FRM, 0x20).await.unwrap(), vec![1]);
        c.write_record(&bmw_e70::FRM, 0x20, &[0]).await.unwrap();
        assert_eq!(c.read_record(&bmw_e70::FRM, 0x20).await.unwrap(), vec![0]);
        // Two-byte adaptation.
        assert_eq!(
            c.read_record(&bmw_e70::DME, 0x01).await.unwrap(),
            940i16.to_be_bytes().to_vec()
        );
    }

    #[tokio::test]
    async fn record_writes_follow_the_rules() {
        let mut c = client(SimEngine::Idling).await;
        // Coding with the engine running: conditionsNotCorrect.
        let e = c.write_record(&bmw_e70::FRM, 0x20, &[0]).await.unwrap_err();
        assert_eq!(nrc_of(e), 0x22);
        c.link_mut().set_engine(SimEngine::IgnitionOn);
        // Learned values cannot be written.
        let e = c
            .write_record(&bmw_e70::DME, 0x01, &[0, 0])
            .await
            .unwrap_err();
        assert_eq!(nrc_of(e), 0x31);
        // Wrong length.
        let e = c
            .write_record(&bmw_e70::FRM, 0x20, &[0, 0])
            .await
            .unwrap_err();
        assert_eq!(nrc_of(e), 0x13);
        // Unknown record.
        let e = c.read_record(&bmw_e70::FRM, 0x7F).await.unwrap_err();
        assert_eq!(nrc_of(e), 0x31);
    }

    #[tokio::test]
    async fn adaptation_reset_moves_trims_from_long_to_short_term() {
        let mut c = client(SimEngine::Idling).await;
        let ltft = c.read_pid(0x07).await.unwrap();
        assert!((ltft - 9.4).abs() < 1.0, "{ltft}");
        c.link_mut().reset_adaptations(bmw_e70::DME.address);
        assert_eq!(
            c.read_record(&bmw_e70::DME, 0x01).await.unwrap(),
            vec![0, 0]
        );
        let ltft = c.read_pid(0x07).await.unwrap();
        let stft = c.read_pid(0x06).await.unwrap();
        assert!(ltft.abs() < 1.0, "{ltft}");
        assert!(stft > 3.0, "short-term covers the leak now: {stft}");
        // Coding is untouched by an adaptation reset.
        assert_eq!(c.read_record(&bmw_e70::DME, 0x10).await.unwrap(), vec![210]);
    }
}
