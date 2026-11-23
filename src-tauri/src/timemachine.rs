//! Module Time Machine storage: one JSON file per snapshot, grouped by VIN.
//!
//! Snapshots are never deleted by the app. They are small (a few hundred
//! bytes each), and a backup that might be needed later is worth more than
//! the disk space.

use protocol_core::bmw_e70;
use protocol_core::snapshot::{RecordValue, Snapshot, SnapshotFault, SnapshotKind};
use std::fs;
use std::io;
use std::path::PathBuf;

pub struct SnapshotStore {
    dir: PathBuf,
}

impl SnapshotStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn vin_dir(&self, vin: &str) -> PathBuf {
        // VINs are 17 alphanumerics; anything else is reduced to a safe name.
        let safe: String = vin
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(32)
            .collect();
        self.dir.join(if safe.is_empty() {
            "UNKNOWN".into()
        } else {
            safe
        })
    }

    pub fn save(&self, s: &Snapshot) -> io::Result<()> {
        let dir = self.vin_dir(&s.vin);
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", s.id));
        let tmp = dir.join(format!(".{}.tmp", s.id));
        fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
        fs::rename(tmp, path)
    }

    /// All snapshots for a car, newest first. Unreadable files are skipped.
    pub fn list(&self, vin: &str) -> Vec<Snapshot> {
        let Ok(entries) = fs::read_dir(self.vin_dir(vin)) else {
            return Vec::new();
        };
        let mut out: Vec<Snapshot> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| fs::read(e.path()).ok())
            .filter_map(|b| serde_json::from_slice(&b).ok())
            .collect();
        out.sort_by_key(|s| std::cmp::Reverse(s.taken_at_ms));
        out
    }

    pub fn get(&self, vin: &str, id: &str) -> Option<Snapshot> {
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return None;
        }
        let bytes = fs::read(self.vin_dir(vin).join(format!("{id}.json"))).ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

/// When, why, which module, its records, its fault codes.
type DemoEntry = (
    u64,
    &'static str,
    bmw_e70::Module,
    Vec<(u8, Vec<u8>)>,
    Vec<&'static str>,
);

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// Gives the demo car a past, so comparing has something to show: a visit
/// to a shop 34 days ago and a check 12 days ago. Values are invented and
/// every snapshot is marked as demo history. Only runs once per store.
pub fn seed_demo_history(store: &SnapshotStore, vin: &str, now_ms: u64) -> io::Result<()> {
    if !store.list(vin).is_empty() {
        return Ok(());
    }
    let w = |v: i16| v.to_be_bytes().to_vec();
    let one = |v: u8| vec![v];
    let (dme, egs, frm, cas, kombi) = (
        bmw_e70::DME,
        bmw_e70::EGS,
        bmw_e70::FRM,
        bmw_e70::CAS,
        bmw_e70::KOMBI,
    );
    let shop = now_ms - 34 * DAY_MS - 3 * 60 * 60 * 1000;
    let check = now_ms - 12 * DAY_MS - 5 * 60 * 60 * 1000;
    let history: Vec<DemoEntry> = vec![
        (
            shop,
            "Before shop visit",
            dme,
            vec![
                (0x01, w(160)),
                (0x02, w(120)),
                (0x03, w(6)),
                (0x04, w(-6)),
                (0x05, w(58)),
                (0x10, one(210)),
                (0x11, one(0)),
            ],
            vec![],
        ),
        (
            shop,
            "Before shop visit",
            egs,
            vec![(0x30, w(62)), (0x31, w(40)), (0x32, w(1702))],
            vec![],
        ),
        (
            shop,
            "Before shop visit",
            frm,
            vec![(0x20, one(0)), (0x21, one(3)), (0x22, one(0))],
            vec![],
        ),
        (
            shop,
            "Before shop visit",
            cas,
            vec![(0x40, one(1)), (0x41, one(1))],
            vec![],
        ),
        (
            shop,
            "Before shop visit",
            kombi,
            vec![(0x50, one(0)), (0x51, one(0))],
            vec![],
        ),
        (
            check,
            "Monthly check",
            dme,
            vec![
                (0x01, w(510)),
                (0x02, w(440)),
                (0x03, w(11)),
                (0x04, w(-11)),
                (0x05, w(60)),
                (0x10, one(210)),
                (0x11, one(0)),
            ],
            vec!["29F3"],
        ),
        (
            check,
            "Monthly check",
            egs,
            vec![(0x30, w(71)), (0x31, w(41)), (0x32, w(1796))],
            vec![],
        ),
    ];
    for (at, reason, module, records, faults) in history {
        store.save(&Snapshot {
            id: Snapshot::make_id(at, module.address),
            vin: vin.into(),
            module_address: module.address,
            module_name: module.name.into(),
            taken_at_ms: at,
            kind: SnapshotKind::Demo,
            reason: format!("{reason} (demo history)"),
            simulated: true,
            identification: Some(format!("SIM {} 0000000 HW01 SW01", module.name)),
            records: records
                .into_iter()
                .map(|(lid, raw)| RecordValue { lid, raw })
                .collect(),
            faults: faults
                .into_iter()
                .map(|c| SnapshotFault {
                    code: c.into(),
                    status: 0x24,
                })
                .collect(),
            missing: vec![],
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> SnapshotStore {
        let dir = std::env::temp_dir().join(format!(
            "apexdiag-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        SnapshotStore::new(dir)
    }

    #[test]
    fn seeds_once_and_lists_newest_first() {
        let store = temp_store();
        let now = 1_800_000_000_000;
        seed_demo_history(&store, "SIMULATED0APEX001", now).unwrap();
        let all = store.list("SIMULATED0APEX001");
        assert_eq!(all.len(), 7);
        assert!(all.windows(2).all(|p| p[0].taken_at_ms >= p[1].taken_at_ms));
        assert!(all.iter().all(|s| s.kind == SnapshotKind::Demo));

        // Seeding again does nothing.
        seed_demo_history(&store, "SIMULATED0APEX001", now).unwrap();
        assert_eq!(store.list("SIMULATED0APEX001").len(), 7);

        let first = &all[0];
        assert_eq!(store.get(&first.vin, &first.id).as_ref(), Some(first));
        assert_eq!(store.get(&first.vin, "../../etc/passwd"), None);
        assert!(store.list("OTHERVIN").is_empty());
    }
}
