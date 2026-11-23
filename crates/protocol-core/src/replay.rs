//! Instant Replay: a rolling memory of live data, and a plain-language
//! summary of what moved around a moment the driver marked.
//!
//! Intermittent faults (a stumble, a shudder, a light that flashes and goes
//! out) are gone by the time a tester is plugged in. The app keeps the last
//! minute of every streamed value; when the driver presses Mark, the window
//! around that moment is saved and compared with the calm period before it.
//!
//! The summary only states what the numbers did ("Engine speed dropped from
//! 702 to 521 rpm, 1.4 s before you pressed"). It never guesses a cause.

use crate::obd2;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// How much history is kept before a mark.
pub const BEFORE_MS: u64 = 30_000;
/// How long recording continues after a mark.
pub const AFTER_MS: u64 = 30_000;
/// The buffer holds a little more than one full replay.
pub const KEEP_MS: u64 = BEFORE_MS + AFTER_MS + 5_000;

/// One poll of the live data: every value read at about the same moment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// Milliseconds since the Unix epoch.
    pub t_ms: u64,
    pub values: Vec<(u8, f64)>,
}

/// Rolling live-data history, pruned to [`KEEP_MS`].
#[derive(Debug, Default)]
pub struct Buffer {
    frames: VecDeque<Frame>,
}

impl Buffer {
    pub fn push(&mut self, t_ms: u64, values: Vec<(u8, f64)>) {
        if values.is_empty() {
            return;
        }
        self.frames.push_back(Frame { t_ms, values });
        while self.frames.front().is_some_and(|f| f.t_ms + KEEP_MS < t_ms) {
            self.frames.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Frames with `from <= t <= to`, oldest first.
    pub fn window(&self, from: u64, to: u64) -> Vec<Frame> {
        self.frames
            .iter()
            .filter(|f| f.t_ms >= from && f.t_ms <= to)
            .cloned()
            .collect()
    }

    /// The window a replay marked at `mark_ms` covers.
    pub fn replay_window(&self, mark_ms: u64) -> Vec<Frame> {
        self.window(mark_ms.saturating_sub(BEFORE_MS), mark_ms + AFTER_MS)
    }
}

/// One value that moved noticeably around the mark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub pid: u8,
    pub name: String,
    pub unit: String,
    /// What the value read around the peak's moment (the median of the
    /// readings within a few seconds of it).
    pub usual: f64,
    /// The furthest the value went near the mark.
    pub peak: f64,
    /// When the peak happened, relative to the mark (negative = before).
    pub offset_ms: i64,
    /// How unusual the move was compared with the value's normal wobble.
    pub score: f64,
    pub text: String,
}

/// The period that counts as "just before" the mark (and a moment after,
/// because people press a little late).
pub const FOCUS_BEFORE_MS: u64 = 8_000;
pub const FOCUS_AFTER_MS: u64 = 3_000;
/// Half-width of the neighbourhood a reading is compared with.
pub const LOCAL_MS: u64 = 3_000;

/// Smallest move worth reporting for a unit, so steady values that wobble
/// by a hair are not flagged.
fn min_move(unit: &str, range: f64) -> f64 {
    match unit {
        "rpm" => 80.0,
        "%" => 3.0,
        "°C" => 2.0,
        "V" => 0.15,
        "kPa" => 4.0,
        "g/s" => 0.6,
        "km/h" => 3.0,
        "°" => 2.0,
        "λ" => 0.03,
        _ => range * 0.03,
    }
}

/// Rounds for display, without ever showing "-0.0".
fn round(v: f64, digits: usize) -> f64 {
    let f = 10f64.powi(digits as i32);
    let r = (v * f).round() / f;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

fn digits(unit: &str) -> usize {
    match unit {
        "V" | "λ" => 2,
        "%" | "g/s" | "°" => 1,
        _ => 0,
    }
}

/// Values that moved around `mark_ms`, most unusual first.
///
/// The calm period is everything from the start of the window up to
/// [`FOCUS_BEFORE_MS`] before the mark. Each value's peak in the focus
/// period is compared with the readings around it, and with how much the
/// value normally strays from its neighbours in the calm period.
pub fn what_changed(frames: &[Frame], mark_ms: u64) -> Vec<Change> {
    let focus_from = mark_ms.saturating_sub(FOCUS_BEFORE_MS);
    let focus_to = mark_ms + FOCUS_AFTER_MS;
    let mut pids: Vec<u8> = frames
        .iter()
        .flat_map(|f| f.values.iter().map(|(p, _)| *p))
        .collect();
    pids.sort_unstable();
    pids.dedup();

    let mut out = Vec::new();
    for pid in pids {
        let all: Vec<(u64, f64)> = frames
            .iter()
            .filter_map(|f| {
                f.values
                    .iter()
                    .find(|(p, _)| *p == pid)
                    .map(|(_, v)| (f.t_ms, *v))
            })
            .collect();
        // Each reading is compared with the median of the readings around
        // it. Slow drifts and gentle swings (coolant warming up, a
        // thermostat cycling) follow the median, so only sudden moves stand
        // out.
        let baseline: Vec<f64> = all
            .iter()
            .map(|&(t, _)| {
                let mut near: Vec<f64> = all
                    .iter()
                    .filter(|(u, _)| u.abs_diff(t) <= LOCAL_MS)
                    .map(|(_, v)| *v)
                    .collect();
                near.sort_by(f64::total_cmp);
                near[near.len() / 2]
            })
            .collect();
        let residual = |i: usize| all[i].1 - baseline[i];
        let calm: Vec<usize> = (0..all.len()).filter(|&i| all[i].0 < focus_from).collect();
        let focus: Vec<usize> = (0..all.len())
            .filter(|&i| all[i].0 >= focus_from && all[i].0 <= focus_to)
            .collect();
        if calm.len() < 5 || focus.is_empty() {
            continue;
        }
        let spread =
            (calm.iter().map(|&i| residual(i).powi(2)).sum::<f64>() / calm.len() as f64).sqrt();
        let (name, unit, range) = match obd2::pid_info(pid) {
            Some(i) => (i.name.to_string(), i.unit.to_string(), i.max - i.min),
            None => (format!("PID {pid:02X}"), String::new(), 100.0),
        };
        let floor = min_move(&unit, range);
        let Some(&i) = focus
            .iter()
            .max_by(|&&a, &&b| residual(a).abs().total_cmp(&residual(b).abs()))
        else {
            continue;
        };
        let (t, peak) = all[i];
        let mean = round(baseline[i], digits(&unit));
        let moved = residual(i).abs();
        if moved < floor {
            continue;
        }
        // A value that already swings widely (an oxygen sensor) needs a
        // bigger move to count.
        let score = moved / spread.max(floor / 3.0);
        if score < 3.0 {
            continue;
        }
        let offset_ms = t as i64 - mark_ms as i64;
        let d = digits(&unit);
        let peak = round(peak, d);
        let when = if offset_ms.abs() < 250 {
            "as you pressed".to_string()
        } else if offset_ms < 0 {
            format!("{:.1} s before you pressed", -offset_ms as f64 / 1000.0)
        } else {
            format!("{:.1} s after you pressed", offset_ms as f64 / 1000.0)
        };
        let verb = if peak > mean { "rose" } else { "dropped" };
        let sep = if unit.is_empty() || unit == "%" || unit == "°" || unit == "°C" {
            ""
        } else {
            " "
        };
        let text = format!(
            "{name} {verb} from about {mean:.d$}{sep}{unit} to {peak:.d$}{sep}{unit}, {when}."
        );
        out.push(Change {
            pid,
            name,
            unit,
            usual: mean,
            peak,
            offset_ms,
            score,
            text,
        });
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 8 frames a second of an idling engine, with a stumble 2 s before the
    /// mark at t = 40 s.
    fn idle_with_stumble() -> Vec<Frame> {
        (0..480u64)
            .map(|i| {
                let t = i * 125;
                let s = t as f64 / 1000.0;
                let stumble = (38_000..38_800).contains(&t);
                let rpm = 700.0 + 20.0 * (s * 1.7).sin() - if stumble { 190.0 } else { 0.0 };
                let stft = 1.5 * (s * 2.1).sin() + if stumble { 11.0 } else { 0.0 };
                let o2 = 0.45 + 0.4 * (s * 4.0).sin();
                // Warming up steadily: a trend, not an event.
                let coolant = 60.0 + 0.5 * s + 1.0 * (s * 0.3).sin();
                Frame {
                    t_ms: t,
                    values: vec![(0x0C, rpm), (0x06, stft), (0x14, o2), (0x05, coolant)],
                }
            })
            .collect()
    }

    #[test]
    fn finds_the_stumble_and_ignores_normal_wobble() {
        let changes = what_changed(&idle_with_stumble(), 40_000);
        let pids: Vec<u8> = changes.iter().map(|c| c.pid).collect();
        assert!(pids.contains(&0x0C), "{changes:?}");
        assert!(pids.contains(&0x06), "{changes:?}");
        // The O2 sensor swings all the time; coolant barely moves.
        assert!(!pids.contains(&0x14), "{changes:?}");
        assert!(!pids.contains(&0x05), "{changes:?}");
        let rpm = changes.iter().find(|c| c.pid == 0x0C).unwrap();
        assert!(rpm.offset_ms < -1_000 && rpm.offset_ms > -2_500);
        assert!(
            (650.0..750.0).contains(&rpm.usual) && rpm.peak < 530.0,
            "{rpm:?}"
        );
        assert!(
            rpm.text.starts_with("Engine speed dropped from about "),
            "{}",
            rpm.text
        );
        assert!(rpm.text.ends_with("s before you pressed."), "{}", rpm.text);
        let trim = changes.iter().find(|c| c.pid == 0x06).unwrap();
        assert!(!trim.text.contains("-0.0"), "{}", trim.text);
    }

    #[test]
    fn calm_data_reports_nothing() {
        let frames: Vec<Frame> = idle_with_stumble()
            .into_iter()
            .filter(|f| !(37_900..38_900).contains(&f.t_ms))
            .collect();
        assert!(what_changed(&frames, 40_000).is_empty());
    }

    #[test]
    fn buffer_keeps_about_a_minute() {
        let mut b = Buffer::default();
        for i in 0..1000u64 {
            b.push(i * 100, vec![(0x0C, 700.0)]);
        }
        let last = 999 * 100;
        assert!(b.window(0, last).first().unwrap().t_ms + KEEP_MS >= last);
        assert_eq!(b.replay_window(last - AFTER_MS).len(), 601);
        b.push(last + 100, vec![]);
        assert_eq!(b.window(last + 100, last + 100).len(), 0);
    }
}
