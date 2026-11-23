//! Diagnostic session bookkeeping: the TesterPresent keep-alive.
//!
//! A module drops back to its default session (and forgets any unlocked
//! security level) if it hears nothing for a few seconds. While a session is
//! active the app sends TesterPresent every 2 seconds. Any other request also
//! counts as traffic, so the timer restarts whenever something is sent.
//!
//! Time is passed in, so this is deterministic and testable.

use crate::diag::{request, Protocol};
use std::time::{Duration, Instant};

/// Default keep-alive interval.
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(2000);

#[derive(Debug, Clone)]
pub struct KeepAlive {
    protocol: Protocol,
    interval: Duration,
    last_traffic: Option<Instant>,
    active: bool,
}

impl KeepAlive {
    pub fn new(protocol: Protocol) -> Self {
        Self {
            protocol,
            interval: KEEPALIVE_INTERVAL,
            last_traffic: None,
            active: false,
        }
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Start keeping the session alive.
    pub fn start(&mut self, now: Instant) {
        self.active = true;
        self.last_traffic = Some(now);
    }

    /// Stop; the module will time out back to its default session.
    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Record that a request was sent.
    pub fn on_traffic(&mut self, now: Instant) {
        self.last_traffic = Some(now);
    }

    /// When the next keep-alive is due, if active.
    pub fn next_due(&self) -> Option<Instant> {
        match (self.active, self.last_traffic) {
            (true, Some(t)) => Some(t + self.interval),
            (true, None) => Some(Instant::now()),
            _ => None,
        }
    }

    /// The TesterPresent request to send now, if one is due.
    pub fn poll(&mut self, now: Instant) -> Option<Vec<u8>> {
        let due = self.next_due()?;
        if now >= due {
            self.last_traffic = Some(now);
            Some(request::tester_present(self.protocol))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_every_interval_only_when_active() {
        let t0 = Instant::now();
        let mut ka = KeepAlive::new(Protocol::Uds);
        assert_eq!(ka.poll(t0 + Duration::from_secs(10)), None);

        ka.start(t0);
        assert_eq!(ka.poll(t0 + Duration::from_millis(1999)), None);
        assert_eq!(
            ka.poll(t0 + Duration::from_millis(2000)),
            Some(vec![0x3E, 0x00])
        );
        assert_eq!(ka.poll(t0 + Duration::from_millis(2500)), None);
        assert!(ka.poll(t0 + Duration::from_millis(4000)).is_some());

        ka.stop();
        assert_eq!(ka.poll(t0 + Duration::from_secs(60)), None);
    }

    #[test]
    fn other_traffic_postpones_keepalive() {
        let t0 = Instant::now();
        let mut ka = KeepAlive::new(Protocol::Kwp2000);
        ka.start(t0);
        ka.on_traffic(t0 + Duration::from_millis(1500));
        assert_eq!(ka.poll(t0 + Duration::from_millis(2000)), None);
        assert_eq!(
            ka.poll(t0 + Duration::from_millis(3500)),
            Some(vec![0x3E, 0x01])
        );
    }
}
