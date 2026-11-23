//! A raw capture of everything sent to and received from an adapter.
//!
//! [`Capture`] wraps any [`HardwareTransport`] and appends each exchange to a
//! text log, with the time since the capture started. The log is what lets
//! real-car data be checked afterwards: which modules answered, what they
//! said, and where an exchange went wrong. It never changes what is sent.
//!
//! ```text
//! # ApexDiag capture, adapter "Veepeak OBDCheck BLE", started 1795417688963 ms
//! +0.000 TX AT Z\r
//! +0.912 RX \r\rELM327 v2.2\r\r>
//! ```

use crate::{HardwareTransport, Result};
use async_trait::async_trait;
use std::io::Write;
use std::time::{Duration, Instant};

/// Where capture lines go: a file in the app's data folder, or memory in
/// tests.
pub trait Sink: Send {
    fn line(&mut self, line: &str);
}

impl Sink for std::fs::File {
    fn line(&mut self, line: &str) {
        // A failed log write must never break the diagnostic session.
        let _ = writeln!(self, "{line}");
        let _ = self.flush();
    }
}

impl Sink for Vec<String> {
    fn line(&mut self, line: &str) {
        self.push(line.to_string());
    }
}

/// Makes control characters visible, so `\r` and `>` line up with the
/// adapter's own framing when the log is read back.
pub fn escape(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            b'\r' => out.push_str("\\r"),
            b'\n' => out.push_str("\\n"),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7E => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02X}")),
        }
    }
    out
}

pub struct Capture<T, S: Sink = std::fs::File> {
    inner: T,
    sink: S,
    start: Instant,
}

impl<T: HardwareTransport, S: Sink> Capture<T, S> {
    pub fn new(inner: T, mut sink: S, adapter: &str, started_ms: u128) -> Self {
        sink.line(&format!(
            "# ApexDiag capture, adapter \"{adapter}\", started {started_ms} ms"
        ));
        sink.line("# TX = sent to the adapter, RX = received. Times are seconds since the start.");
        Self {
            inner,
            sink,
            start: Instant::now(),
        }
    }

    /// Adds a note line, such as which test step is running.
    pub fn note(&mut self, text: &str) {
        let t = self.start.elapsed().as_secs_f64();
        self.sink.line(&format!("+{t:.3} -- {text}"));
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    fn log(&mut self, dir: &str, bytes: &[u8]) {
        let t = self.start.elapsed().as_secs_f64();
        self.sink.line(&format!("+{t:.3} {dir} {}", escape(bytes)));
    }
}

#[async_trait]
impl<T: HardwareTransport, S: Sink> HardwareTransport for Capture<T, S> {
    async fn connect(&mut self) -> Result<()> {
        let r = self.inner.connect().await;
        match &r {
            Ok(()) => self.note("link open"),
            Err(e) => self.note(&format!("link failed: {e}")),
        }
        r
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.note("link closed");
        self.inner.disconnect().await
    }

    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    async fn send(&mut self, bytes: &[u8]) -> Result<()> {
        self.log("TX", bytes);
        let r = self.inner.send(bytes).await;
        if let Err(e) = &r {
            self.note(&format!("send failed: {e}"));
        }
        r
    }

    async fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let r = self.inner.recv(timeout).await;
        match &r {
            Ok(bytes) if !bytes.is_empty() => self.log("RX", bytes),
            Ok(_) | Err(crate::TransportError::Timeout(_)) => {}
            Err(e) => self.note(&format!("receive failed: {e}")),
        }
        r
    }

    fn simulator(&mut self) -> Option<&mut crate::sim::ElmSimulator> {
        self.inner.simulator()
    }

    fn capture_note(&mut self, text: &str) {
        self.note(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elm::ElmClient;
    use crate::sim::{ElmSimulator, SimEngine};

    #[test]
    fn escapes_control_bytes() {
        assert_eq!(escape(b"41 0C\r\r>"), "41 0C\\r\\r>");
        assert_eq!(escape(&[0x00, b'\\']), "\\x00\\\\");
    }

    #[tokio::test]
    async fn logs_every_exchange_without_changing_it() {
        let link = Capture::new(ElmSimulator::new(SimEngine::Idling), Vec::new(), "sim", 0);
        let mut c = ElmClient::new(link);
        c.connect().await.unwrap();
        let rpm = c.read_pid(0x0C).await.unwrap();
        assert!(rpm > 500.0);
        c.link_mut().capture_note("step: live data");
        let log = c.link_mut().sink().clone();
        assert!(log[0].starts_with("# ApexDiag capture"));
        assert!(log.iter().any(|l| l.contains("TX AT Z\\r")));
        assert!(log
            .iter()
            .any(|l| l.contains("RX ") && l.contains("ELM327")));
        assert!(log.iter().any(|l| l.contains("TX 01 0C\\r")));
        assert!(log.iter().any(|l| l.ends_with("-- step: live data")));
    }
}
