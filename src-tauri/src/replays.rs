//! Saved Instant Replays: one JSON file per replay, grouped by VIN, next to
//! the repair records.

use protocol_core::replay::{Change, Frame};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayCode {
    pub code: String,
    pub status: Option<u8>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayModule {
    pub name: String,
    pub address: u8,
    pub present: bool,
    pub codes: Vec<ReplayCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replay {
    pub id: String,
    pub vin: String,
    /// "BMW X5 (E70)" etc.
    pub vehicle: String,
    pub simulated: bool,
    /// When the driver pressed Mark (ms since the Unix epoch).
    pub mark_ms: u64,
    pub note: String,
    pub frames: Vec<Frame>,
    pub changes: Vec<Change>,
    /// Engine OBD-II codes read right after the window.
    pub engine_stored: Vec<ReplayCode>,
    pub engine_pending: Vec<ReplayCode>,
    /// Fault memory of every module the app may scan on this car; empty if
    /// the module scan was skipped.
    pub modules: Vec<ReplayModule>,
}

/// What the list shows, without the frames.
#[derive(Debug, Clone, Serialize)]
pub struct ReplaySummary {
    pub id: String,
    pub vin: String,
    pub vehicle: String,
    pub simulated: bool,
    pub mark_ms: u64,
    pub note: String,
    pub seconds: f64,
    pub headline: Option<String>,
    pub changes: usize,
    pub faults: usize,
}

impl Replay {
    pub fn summary(&self) -> ReplaySummary {
        let span = match (self.frames.first(), self.frames.last()) {
            (Some(a), Some(b)) => (b.t_ms - a.t_ms) as f64 / 1000.0,
            _ => 0.0,
        };
        ReplaySummary {
            id: self.id.clone(),
            vin: self.vin.clone(),
            vehicle: self.vehicle.clone(),
            simulated: self.simulated,
            mark_ms: self.mark_ms,
            note: self.note.clone(),
            seconds: span,
            headline: self.changes.first().map(|c| c.text.clone()),
            changes: self.changes.len(),
            faults: self.engine_stored.len()
                + self.engine_pending.len()
                + self.modules.iter().map(|m| m.codes.len()).sum::<usize>(),
        }
    }
}

pub struct ReplayStore {
    root: PathBuf,
}

fn safe(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn folder(vin: &str) -> String {
    let clean: String = vin.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if clean.is_empty() {
        "UNKNOWN".into()
    } else {
        clean
    }
}

impl ReplayStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn save(&self, r: &Replay) -> io::Result<()> {
        if !safe(&r.id) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad replay id"));
        }
        let dir = self.root.join(folder(&r.vin));
        fs::create_dir_all(&dir)?;
        let json = serde_json::to_vec(r).map_err(io::Error::other)?;
        fs::write(dir.join(format!("{}.json", r.id)), json)
    }

    fn files(&self) -> Vec<PathBuf> {
        let Ok(dirs) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        dirs.flatten()
            .filter_map(|d| fs::read_dir(d.path()).ok())
            .flat_map(|f| f.flatten().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect()
    }

    fn path(&self, id: &str) -> Option<PathBuf> {
        if !safe(id) {
            return None;
        }
        let name = format!("{id}.json");
        self.files()
            .into_iter()
            .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy() == name))
    }

    pub fn get(&self, id: &str) -> Option<Replay> {
        let bytes = fs::read(self.path(id)?).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn delete(&self, id: &str) -> io::Result<()> {
        match self.path(id) {
            Some(p) => fs::remove_file(p),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "no such replay")),
        }
    }

    /// Every saved replay, newest first.
    pub fn list(&self) -> Vec<ReplaySummary> {
        let mut out: Vec<ReplaySummary> = self
            .files()
            .into_iter()
            .filter_map(|p| fs::read(p).ok())
            .filter_map(|b| serde_json::from_slice::<Replay>(&b).ok())
            .map(|r| r.summary())
            .collect();
        out.sort_by_key(|r| std::cmp::Reverse(r.mark_ms));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replay(id: &str, vin: &str, mark_ms: u64) -> Replay {
        Replay {
            id: id.into(),
            vin: vin.into(),
            vehicle: "BMW X5 (E70)".into(),
            simulated: true,
            mark_ms,
            note: String::new(),
            frames: vec![
                Frame {
                    t_ms: mark_ms - 1000,
                    values: vec![(0x0C, 700.0)],
                },
                Frame {
                    t_ms: mark_ms + 1000,
                    values: vec![(0x0C, 520.0)],
                },
            ],
            changes: vec![],
            engine_stored: vec![],
            engine_pending: vec![ReplayCode {
                code: "P0300".into(),
                status: None,
                description: None,
            }],
            modules: vec![],
        }
    }

    #[test]
    fn saves_lists_and_deletes() {
        let root = std::env::temp_dir().join(format!("apexdiag-replays-{}", std::process::id()));
        let store = ReplayStore::new(root.clone());
        store.save(&replay("x1", "WBAFE41", 5_000)).unwrap();
        store.save(&replay("x2", "", 9_000)).unwrap();
        assert!(store.save(&replay("../x", "A", 2_000)).is_err());
        let list = store.list();
        assert_eq!(
            list.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["x2", "x1"]
        );
        assert_eq!(list[0].seconds, 2.0);
        assert_eq!(list[0].faults, 1);
        assert_eq!(store.get("x1").unwrap().frames.len(), 2);
        assert!(store.get("../x1").is_none());
        store.delete("x1").unwrap();
        assert!(store.get("x1").is_none());
        let _ = fs::remove_dir_all(root);
    }
}
