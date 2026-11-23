//! Repair records: one JSON file per repair, grouped by VIN, next to the
//! Time Machine snapshots. Kept forever, like a service book.

use protocol_core::copilot::RepairRecord;
use std::fs;
use std::io;
use std::path::PathBuf;

pub struct RepairStore {
    root: PathBuf,
}

fn safe(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

impl RepairStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn dir(&self, vin: &str) -> PathBuf {
        let clean: String = vin.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        self.root.join(if clean.is_empty() {
            "UNKNOWN".into()
        } else {
            clean
        })
    }

    pub fn save(&self, r: &RepairRecord) -> io::Result<()> {
        if !safe(&r.id) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad repair id"));
        }
        let dir = self.dir(&r.vin);
        fs::create_dir_all(&dir)?;
        let json = serde_json::to_vec_pretty(r).map_err(io::Error::other)?;
        fs::write(dir.join(format!("{}.json", r.id)), json)
    }

    /// Newest first.
    pub fn list(&self, vin: &str) -> Vec<RepairRecord> {
        let Ok(entries) = fs::read_dir(self.dir(vin)) else {
            return Vec::new();
        };
        let mut out: Vec<RepairRecord> = entries
            .flatten()
            .filter_map(|e| fs::read(e.path()).ok())
            .filter_map(|b| serde_json::from_slice(&b).ok())
            .collect();
        out.sort_by_key(|r| std::cmp::Reverse(r.finished_ms));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_lists_newest_first() {
        let root = std::env::temp_dir().join(format!("apexdiag-repairs-{}", std::process::id()));
        let store = RepairStore::new(root);
        let rec = |id: &str, at: u64| RepairRecord {
            id: id.into(),
            vin: "SIMULATED0APEX001".into(),
            code: "P0171".into(),
            module: None,
            title: "Engine running lean".into(),
            started_ms: at.saturating_sub(1000),
            finished_ms: at,
            steps: vec![],
            fix: "Replaced crankcase vent valve".into(),
            parts: "Crankcase vent valve".into(),
            verification: vec![],
            verified: true,
            simulated: true,
        };
        store.save(&rec("r-1", 10_000)).unwrap();
        store.save(&rec("r-2", 20_000)).unwrap();
        assert!(store.save(&rec("../evil", 1)).is_err());
        let all = store.list("SIMULATED0APEX001");
        assert_eq!(
            all.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["r-2", "r-1"]
        );
        assert!(store.list("OTHER").is_empty());
    }
}
