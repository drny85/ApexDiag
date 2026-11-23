//! Module Time Machine: snapshots of a module's state, comparison between
//! two points in time, and the plan for writing a module back.
//!
//! A snapshot holds what can be read from one module: its identification,
//! its fault memory, and a set of data records. Records come in two kinds:
//!
//! * **Coding**: settings the module was told (lights, locks, limits). These
//!   can be written back, so a restore puts them back exactly.
//! * **Adaptation**: values the module learned by itself (fuel trims, clutch
//!   fill pressures). These are compared to spot drift, but never written
//!   back: the module relearns them, and forcing old values would be wrong.
//!
//! Everything here is pure: reading and writing happen in the transport and
//! app layers.

use serde::{Deserialize, Serialize};

use crate::vehicle::JobStatus;

/// Coding (writable) or adaptation (learned, compare only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    Coding,
    Adaptation,
}

/// How a record's raw bytes turn into something readable.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Format {
    /// One byte picking one of several named options.
    Choice {
        options: &'static [(u8, &'static str)],
    },
    /// A big-endian number of 1 or 2 bytes, times `scale`.
    Number {
        bytes: u8,
        signed: bool,
        scale: f64,
        unit: &'static str,
        /// Decimal places to show.
        digits: u8,
    },
}

/// A record the app knows how to read from a module.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RecordDef {
    /// Module address.
    pub module: u8,
    /// Record identifier: a KWP2000 local id (`21 id` / `3B id`, one byte)
    /// or a UDS data identifier (`22 id` / `2E id`, two bytes).
    pub id: u16,
    pub name: &'static str,
    pub kind: RecordKind,
    pub format: Format,
    /// What the value means and why a change matters.
    pub explain: &'static str,
    /// Whether the identifier is confirmed on a real E70. Unverified
    /// records are only read from the demo car.
    pub status: JobStatus,
}

impl RecordDef {
    /// Expected length of the raw value.
    pub fn len(&self) -> usize {
        match self.format {
            Format::Choice { .. } => 1,
            Format::Number { bytes, .. } => bytes as usize,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Numeric value, for drift and deltas.
    pub fn number(&self, raw: &[u8]) -> Option<f64> {
        if raw.len() != self.len() {
            return None;
        }
        match self.format {
            Format::Choice { .. } => Some(raw[0] as f64),
            Format::Number {
                bytes,
                signed,
                scale,
                ..
            } => {
                let v = match (bytes, signed) {
                    (1, false) => raw[0] as f64,
                    (1, true) => raw[0] as i8 as f64,
                    (2, false) => u16::from_be_bytes([raw[0], raw[1]]) as f64,
                    (2, true) => i16::from_be_bytes([raw[0], raw[1]]) as f64,
                    _ => return None,
                };
                Some(v * scale)
            }
        }
    }

    /// Readable text, e.g. `On`, `+7.80 %`, `5 flashes`.
    pub fn display(&self, raw: &[u8]) -> String {
        if raw.len() != self.len() {
            return format!("Unreadable ({})", hex(raw));
        }
        match self.format {
            Format::Choice { options } => options
                .iter()
                .find(|(v, _)| *v == raw[0])
                .map(|(_, t)| (*t).to_string())
                .unwrap_or_else(|| format!("Unknown option 0x{:02X}", raw[0])),
            Format::Number {
                signed,
                unit,
                digits,
                ..
            } => {
                let v = self.number(raw).unwrap_or_default();
                let sign = if signed && v > 0.0 { "+" } else { "" };
                let n = format!("{sign}{v:.prec$}", prec = digits as usize);
                if unit.is_empty() {
                    n
                } else {
                    format!("{n} {unit}")
                }
            }
        }
    }
}

fn hex(raw: &[u8]) -> String {
    raw.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Why a snapshot was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotKind {
    /// The user pressed "Take snapshot".
    Manual,
    /// Taken by the app right before a write.
    Automatic,
    /// Invented history that ships with the demo car.
    Demo,
}

/// One record's raw value as read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordValue {
    #[serde(alias = "lid")]
    pub id: u16,
    pub raw: Vec<u8>,
}

/// One fault memory entry as read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotFault {
    pub code: String,
    pub status: u8,
}

/// A module's state at one moment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique per VIN: `<taken_at_ms>-<address>`.
    pub id: String,
    pub vin: String,
    pub module_address: u8,
    pub module_name: String,
    pub taken_at_ms: u64,
    pub kind: SnapshotKind,
    /// Plain-language reason, e.g. "Before Reset engine adaptations".
    pub reason: String,
    /// Taken from the demo car.
    pub simulated: bool,
    pub identification: Option<String>,
    pub records: Vec<RecordValue>,
    pub faults: Vec<SnapshotFault>,
    /// Records that were expected but could not be read.
    pub missing: Vec<u16>,
}

impl Snapshot {
    pub fn make_id(taken_at_ms: u64, address: u8) -> String {
        format!("{taken_at_ms}-{address:02x}")
    }

    pub fn record(&self, id: u16) -> Option<&[u8]> {
        self.records
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.raw.as_slice())
    }

    /// True when every expected record was read. A write is only allowed
    /// behind a complete backup.
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }
}

/// How one record differs between two snapshots.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecordChange {
    pub id: u16,
    pub name: &'static str,
    pub kind: RecordKind,
    pub explain: &'static str,
    pub before: Option<String>,
    pub after: Option<String>,
    pub before_raw: Option<Vec<u8>>,
    pub after_raw: Option<Vec<u8>>,
    /// Numeric values, for charts. Choices give their option number.
    pub before_value: Option<f64>,
    pub after_value: Option<f64>,
    /// `after - before`, for numeric records.
    pub delta: Option<f64>,
    pub changed: bool,
}

/// Fault memory differences.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FaultChanges {
    /// Present now, absent before.
    pub appeared: Vec<String>,
    /// Present before, absent now (fixed or cleared).
    pub gone: Vec<String>,
    pub unchanged: Vec<String>,
}

/// Everything that differs between two snapshots of one module.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModuleDiff {
    pub records: Vec<RecordChange>,
    pub faults: FaultChanges,
    pub identification_changed: bool,
    /// Coding records that differ: what a restore of `from` would write.
    pub restorable: usize,
    pub changed: usize,
}

/// Compares `from` (older) to `to` (newer) using `defs` for this module.
pub fn diff(from: &Snapshot, to: &Snapshot, defs: &[RecordDef]) -> ModuleDiff {
    let records: Vec<RecordChange> = defs
        .iter()
        .filter(|d| d.module == from.module_address)
        .map(|d| {
            let a = from.record(d.id);
            let b = to.record(d.id);
            let (va, vb) = (a.and_then(|r| d.number(r)), b.and_then(|r| d.number(r)));
            let delta = match (va, vb) {
                (Some(x), Some(y)) if matches!(d.format, Format::Number { .. }) => Some(y - x),
                _ => None,
            };
            RecordChange {
                id: d.id,
                name: d.name,
                kind: d.kind,
                explain: d.explain,
                before: a.map(|r| d.display(r)),
                after: b.map(|r| d.display(r)),
                before_raw: a.map(<[u8]>::to_vec),
                after_raw: b.map(<[u8]>::to_vec),
                before_value: va,
                after_value: vb,
                delta,
                changed: a != b,
            }
        })
        .collect();

    let codes = |s: &Snapshot| s.faults.iter().map(|f| f.code.clone()).collect::<Vec<_>>();
    let (old, new) = (codes(from), codes(to));
    let faults = FaultChanges {
        appeared: new.iter().filter(|c| !old.contains(c)).cloned().collect(),
        gone: old.iter().filter(|c| !new.contains(c)).cloned().collect(),
        unchanged: new.iter().filter(|c| old.contains(c)).cloned().collect(),
    };

    let restorable = records
        .iter()
        .filter(|r| r.kind == RecordKind::Coding && r.changed && r.before_raw.is_some())
        .count();
    let changed = records.iter().filter(|r| r.changed).count();
    ModuleDiff {
        identification_changed: from.identification != to.identification,
        records,
        faults,
        restorable,
        changed,
    }
}

/// One write needed to put a module's coding back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Write {
    pub id: u16,
    pub name: &'static str,
    pub from: String,
    pub to: String,
    pub raw: Vec<u8>,
}

/// The coding writes that turn `current` back into `target`. Adaptations
/// and fault memory are never written. Values that already match are
/// skipped, so restoring twice writes nothing the second time.
pub fn restore_plan(current: &Snapshot, target: &Snapshot, defs: &[RecordDef]) -> Vec<Write> {
    defs.iter()
        .filter(|d| d.module == target.module_address && d.kind == RecordKind::Coding)
        .filter_map(|d| {
            let want = target.record(d.id)?;
            if want.len() != d.len() {
                return None;
            }
            let have = current.record(d.id);
            (have != Some(want)).then(|| Write {
                id: d.id,
                name: d.name,
                from: have
                    .map(|h| d.display(h))
                    .unwrap_or_else(|| "Unknown".into()),
                to: d.display(want),
                raw: want.to_vec(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONOFF: &[(u8, &str)] = &[(0, "Off"), (1, "On")];

    const DEFS: &[RecordDef] = &[
        RecordDef {
            module: 0x72,
            id: 0x20,
            name: "Daytime running lights",
            kind: RecordKind::Coding,
            format: Format::Choice { options: ONOFF },
            explain: "",
            status: JobStatus::PendingCapture,
        },
        RecordDef {
            module: 0x72,
            id: 0x21,
            name: "Comfort turn signal",
            kind: RecordKind::Coding,
            format: Format::Number {
                bytes: 1,
                signed: false,
                scale: 1.0,
                unit: "flashes",
                digits: 0,
            },
            explain: "",
            status: JobStatus::PendingCapture,
        },
        RecordDef {
            module: 0x72,
            id: 0x22,
            name: "Fuel trim",
            kind: RecordKind::Adaptation,
            format: Format::Number {
                bytes: 2,
                signed: true,
                scale: 0.01,
                unit: "%",
                digits: 2,
            },
            explain: "",
            status: JobStatus::PendingCapture,
        },
    ];

    fn snap(at: u64, dlr: u8, flashes: u8, trim: i16, faults: &[&str]) -> Snapshot {
        Snapshot {
            id: Snapshot::make_id(at, 0x72),
            vin: "SIMULATED0APEX001".into(),
            module_address: 0x72,
            module_name: "FRM".into(),
            taken_at_ms: at,
            kind: SnapshotKind::Manual,
            reason: String::new(),
            simulated: true,
            identification: Some("FRM HW01".into()),
            records: vec![
                RecordValue {
                    id: 0x20,
                    raw: vec![dlr],
                },
                RecordValue {
                    id: 0x21,
                    raw: vec![flashes],
                },
                RecordValue {
                    id: 0x22,
                    raw: trim.to_be_bytes().to_vec(),
                },
            ],
            faults: faults
                .iter()
                .map(|c| SnapshotFault {
                    code: (*c).into(),
                    status: 0x24,
                })
                .collect(),
            missing: vec![],
        }
    }

    #[test]
    fn displays_values() {
        assert_eq!(DEFS[0].display(&[1]), "On");
        assert_eq!(DEFS[0].display(&[7]), "Unknown option 0x07");
        assert_eq!(DEFS[1].display(&[5]), "5 flashes");
        assert_eq!(DEFS[2].display(&780i16.to_be_bytes()), "+7.80 %");
        assert_eq!(DEFS[2].display(&(-45i16).to_be_bytes()), "-0.45 %");
        assert_eq!(DEFS[2].display(&[1]), "Unreadable (01)");
    }

    #[test]
    fn diff_finds_coding_adaptation_and_fault_changes() {
        let before = snap(1, 0, 3, 160, &["29F3"]);
        let after = snap(2, 1, 5, 780, &["29F3", "2A82"]);
        let d = diff(&before, &after, DEFS);
        assert_eq!(d.changed, 3);
        assert_eq!(d.restorable, 2);
        let trim = d.records.iter().find(|r| r.id == 0x22).unwrap();
        assert!((trim.delta.unwrap() - 6.2).abs() < 1e-9);
        assert_eq!(trim.before.as_deref(), Some("+1.60 %"));
        let dlr = d.records.iter().find(|r| r.id == 0x20).unwrap();
        assert_eq!(dlr.delta, None, "choices have no delta");
        assert_eq!(d.faults.appeared, vec!["2A82"]);
        assert!(d.faults.gone.is_empty());
        assert_eq!(d.faults.unchanged, vec!["29F3"]);
        assert!(!d.identification_changed);
    }

    #[test]
    fn identical_snapshots_have_no_changes() {
        let a = snap(1, 1, 3, 100, &[]);
        let d = diff(&a, &a.clone(), DEFS);
        assert_eq!(d.changed, 0);
        assert_eq!(d.restorable, 0);
    }

    #[test]
    fn restore_writes_only_differing_coding() {
        let target = snap(1, 0, 3, 160, &[]);
        let current = snap(2, 1, 3, 780, &["2A82"]);
        let plan = restore_plan(&current, &target, DEFS);
        assert_eq!(plan.len(), 1, "adaptations and equal values are skipped");
        assert_eq!(plan[0].id, 0x20);
        assert_eq!(plan[0].raw, vec![0]);
        assert_eq!((plan[0].from.as_str(), plan[0].to.as_str()), ("On", "Off"));

        // After the write, the same restore has nothing to do.
        let restored = snap(3, 0, 3, 780, &[]);
        assert!(restore_plan(&restored, &target, DEFS).is_empty());
    }

    #[test]
    fn restore_skips_records_missing_from_the_target() {
        let mut target = snap(1, 0, 3, 160, &[]);
        target.records.retain(|r| r.id != 0x20);
        let current = snap(2, 1, 5, 160, &[]);
        let plan = restore_plan(&current, &target, DEFS);
        assert_eq!(plan.iter().map(|w| w.id).collect::<Vec<_>>(), vec![0x21]);
    }

    #[test]
    fn snapshot_round_trips_through_json() {
        let s = snap(1_700_000_000_000, 1, 5, -45, &["2A82"]);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"kind\":\"manual\""));
        let back: Snapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
        assert!(back.is_complete());
    }
}
