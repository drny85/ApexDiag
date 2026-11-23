//! A scripted in-memory adapter: each `send` queues the matching reply.

use crate::{HardwareTransport, Result, TransportError};
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

#[derive(Default)]
pub struct MockTransport {
    connected: bool,
    replies: HashMap<Vec<u8>, Vec<u8>>,
    pending: VecDeque<Vec<u8>>,
    /// Every request written, in order, for assertions.
    pub sent: Vec<Vec<u8>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// When `request` is sent, answer with `reply`.
    pub fn on(mut self, request: &[u8], reply: &[u8]) -> Self {
        self.replies.insert(request.to_vec(), reply.to_vec());
        self
    }
}

#[async_trait]
impl HardwareTransport for MockTransport {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn send(&mut self, bytes: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        self.sent.push(bytes.to_vec());
        if let Some(reply) = self.replies.get(bytes) {
            self.pending.push_back(reply.clone());
        }
        Ok(())
    }

    async fn recv(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        self.pending
            .pop_front()
            .ok_or(TransportError::Timeout(timeout))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn scripted_reply() {
        let mut t = MockTransport::new().on(b"AT Z\r", b"ELM327 v1.4b\r\r>");
        assert!(t.send(b"AT Z\r").await.is_err());
        t.connect().await.unwrap();
        t.send(b"AT Z\r").await.unwrap();
        let reply = t.recv(Duration::from_millis(10)).await.unwrap();
        assert_eq!(reply, b"ELM327 v1.4b\r\r>");
        assert!(t.recv(Duration::from_millis(10)).await.is_err());
    }
}
