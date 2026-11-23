//! The engine half of the Full Scan: everything the engine module can tell
//! over OBD-II, read with as few requests as possible. Read-only.
//!
//! - Readiness (PID 01) first: one reply carries the check-engine light,
//!   the code count and every monitor.
//! - Live values only for PIDs the car said it supports, up to six per
//!   request.
//! - The freeze frame only when a code is stored or pending.
//! - Battery voltage from the engine module when it reports it, else the
//!   adapter's own measurement (no request to the car).

use crate::elm::ElmClient;
use crate::HardwareTransport;
use protocol_core::full_scan::{EngineCode, LIVE_PIDS};
use protocol_core::obd2::{Dtc, Readiness, Service};

/// What the engine module said.
#[derive(Debug, Clone, Default)]
pub struct EngineRead {
    pub readiness: Option<Readiness>,
    /// None when the engine codes could not be read.
    pub stored: Option<Vec<EngineCode>>,
    pub pending: Vec<EngineCode>,
    pub permanent: Vec<EngineCode>,
    pub freeze_frame: Option<(String, Vec<(u8, f64)>)>,
    pub live: Vec<(u8, f64)>,
    pub battery_v: Option<f64>,
    pub engine_running: Option<bool>,
}

fn codes(list: Vec<Dtc>) -> Vec<EngineCode> {
    list.into_iter()
        .map(|d| EngineCode {
            code: d.code,
            description: d.description.map(Into::into),
        })
        .collect()
}

/// Reads the engine side of a full scan. `supported` is the car's list of
/// supported Service 01 PIDs (from connect); empty means unknown, and then
/// every PID the scan knows is tried.
pub async fn read_engine<T: HardwareTransport>(
    c: &mut ElmClient<T>,
    supported: &[u8],
) -> EngineRead {
    let readiness = c.read_readiness().await.ok();
    let stored = c.read_obd_dtcs(Service::StoredDtcs).await.ok().map(codes);
    let pending = codes(
        c.read_obd_dtcs(Service::PendingDtcs)
            .await
            .unwrap_or_default(),
    );
    let permanent = codes(
        c.read_obd_dtcs(Service::PermanentDtcs)
            .await
            .unwrap_or_default(),
    );
    let has_codes = stored.as_ref().is_some_and(|s| !s.is_empty())
        || !pending.is_empty()
        || readiness
            .as_ref()
            .is_some_and(|r| r.mil_on || r.dtc_count > 0);
    let freeze_frame = if has_codes {
        let only = (!supported.is_empty()).then_some(supported);
        c.read_freeze_frame_of(only)
            .await
            .unwrap_or(None)
            .map(|f| (f.dtc, f.values))
    } else {
        None
    };
    let wanted: Vec<u8> = LIVE_PIDS
        .iter()
        .copied()
        .filter(|p| supported.is_empty() || supported.contains(p))
        .collect();
    let live = c.read_pids(&wanted).await;
    let rpm = live.iter().find(|(p, _)| *p == 0x0C).map(|(_, v)| *v);
    let battery_v = match live.iter().find(|(p, _)| *p == 0x42) {
        Some((_, v)) => Some(*v),
        None => c.battery_voltage().await.ok().map(f64::from),
    };
    EngineRead {
        readiness,
        stored,
        pending,
        permanent,
        freeze_frame,
        live,
        battery_v,
        engine_running: rpm.map(|r| r > 300.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{sim_car, ElmSimulator, SimEngine};

    async fn connected(car: &str, engine: SimEngine) -> ElmClient<ElmSimulator> {
        let mut c = ElmClient::new(ElmSimulator::with_car(sim_car(car).unwrap(), engine));
        c.connect().await.unwrap();
        c
    }

    #[tokio::test]
    async fn bmw_lean_story_reads_in_few_requests() {
        let mut c = connected("bmw_x5", SimEngine::Idling).await;
        let supported = c.supported_pids().await.unwrap();
        let before = c.link_mut().log.len();
        let r = read_engine(&mut c, &supported).await;
        let sent: Vec<_> = c.link_mut().log[before..]
            .iter()
            .filter(|l| !l.starts_with("AT"))
            .cloned()
            .collect();
        assert_eq!(r.engine_running, Some(true));
        assert!(r.readiness.is_some());
        // Live values arrive in batches: far fewer requests than values.
        assert!(r.live.len() >= 15, "{:?}", r.live);
        let live_requests = sent
            .iter()
            .filter(|l| l.starts_with("01 ") && *l != "01 01")
            .count();
        assert!(
            live_requests * 4 <= r.live.len(),
            "{live_requests} requests for {} values: {sent:?}",
            r.live.len()
        );
        // Unsupported PIDs are never asked for (the BMW has no odometer PID).
        assert!(sent.iter().all(|l| !l.contains("A6")), "{sent:?}");
        assert!(r.battery_v.is_some_and(|v| v > 13.0));
    }

    #[tokio::test]
    async fn honda_freeze_frame_is_read_only_when_a_code_is_stored() {
        let mut c = connected("honda_accord", SimEngine::IgnitionOn).await;
        let supported = c.supported_pids().await.unwrap();
        let r = read_engine(&mut c, &supported).await;
        assert_eq!(r.engine_running, Some(false));
        let (code, values) = r.freeze_frame.expect("freeze frame");
        assert_eq!(code, "P0420");
        assert!(!values.is_empty());

        // The Mercedes has no engine code: no freeze-frame requests at all.
        let mut c = connected("mercedes_gle", SimEngine::Idling).await;
        let supported = c.supported_pids().await.unwrap();
        let before = c.link_mut().log.len();
        let r = read_engine(&mut c, &supported).await;
        assert!(r.freeze_frame.is_none());
        assert!(c.link_mut().log[before..]
            .iter()
            .all(|l| !l.starts_with("02")));
    }
}
