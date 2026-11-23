//! Inspection report: a pre-purchase style checkup, graded green, amber or
//! red per area, with a plain explanation of each grade.
//!
//! Grading is done here, from readings the app has already taken, so the
//! rules are the same on every platform and can be tested. The rules only
//! use generic OBD-II facts and the app's own module scan. Nothing here
//! claims a fault's cause, and anything that could not be read is graded
//! "not checked" rather than passed.

use crate::obd2::Readiness;
use crate::vehicle::System;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grade {
    /// Could not be read; never counted as a pass.
    NotChecked,
    Good,
    Attention,
    Problem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Code {
    pub code: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleResult {
    pub name: String,
    pub system: System,
    pub present: bool,
    pub faults: Vec<Code>,
}

/// Everything the app read for the report.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Readings {
    pub mil_on: Option<bool>,
    /// None when the engine codes could not be read.
    pub stored: Option<Vec<Code>>,
    pub pending: Vec<Code>,
    pub permanent: Vec<Code>,
    pub readiness: Option<Readiness>,
    pub km_since_clear: Option<f64>,
    pub warmups_since_clear: Option<f64>,
    pub km_with_mil: Option<f64>,
    pub battery_v: Option<f64>,
    pub engine_running: Option<bool>,
    pub odometer_km: Option<f64>,
    pub modules: Vec<ModuleResult>,
    /// Modules not asked because their address isn't confirmed for a real
    /// car of this kind yet.
    pub modules_not_scanned: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Section {
    pub id: &'static str,
    pub title: &'static str,
    pub grade: Grade,
    pub headline: String,
    pub details: Vec<String>,
    /// Why this area matters, for someone who isn't a mechanic.
    pub why: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The worst grade among the areas that were checked.
    pub overall: Grade,
    pub summary: String,
    pub sections: Vec<Section>,
}

fn list(codes: &[Code]) -> Vec<String> {
    codes
        .iter()
        .map(|c| match &c.description {
            Some(d) => format!("{}: {d}", c.code),
            None => format!("{}: no description in the app's library yet", c.code),
        })
        .collect()
}

fn engine(r: &Readings) -> Section {
    let why = "The engine module stores a code whenever a self-test fails. Stored codes and a lit engine light are the first thing to check on any car.";
    let Some(stored) = &r.stored else {
        return Section {
            id: "engine",
            title: "Engine light and engine codes",
            grade: Grade::NotChecked,
            headline: "The engine codes could not be read.".into(),
            details: vec![],
            why,
        };
    };
    let mut details = list(stored);
    details.extend(
        list(&r.pending)
            .into_iter()
            .map(|d| format!("{d} (pending)")),
    );
    details.extend(
        list(&r.permanent)
            .into_iter()
            .map(|d| format!("{d} (permanent: clears itself only after the car proves the fix)")),
    );
    let mil = r.mil_on == Some(true);
    let (grade, headline) = if mil || !stored.is_empty() || !r.permanent.is_empty() {
        let n = stored.len().max(r.permanent.len());
        (
            Grade::Problem,
            match (mil, n) {
                (true, 0) => "The engine light is on.".to_string(),
                (true, n) => format!(
                    "The engine light is on with {n} stored {}.",
                    plural(n, "code", "codes")
                ),
                (false, n) => format!(
                    "{n} stored {} with the engine light off.",
                    plural(n, "code", "codes")
                ),
            },
        )
    } else if !r.pending.is_empty() {
        (
            Grade::Attention,
            format!(
                "No stored codes, but {} pending: a test failed once and is being watched.",
                r.pending.len()
            ),
        )
    } else {
        (
            Grade::Good,
            "Engine light off and no codes stored or pending.".into(),
        )
    };
    Section {
        id: "engine",
        title: "Engine light and engine codes",
        grade,
        headline,
        details,
        why,
    }
}

/// 48877.3 -> "48,877".
fn whole(v: f64) -> String {
    let digits = format!("{:.0}", v.abs());
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if v < -0.5 {
        out.insert(0, '-');
    }
    out
}

fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n == 1 {
        one
    } else {
        many
    }
}

fn readiness(r: &Readings) -> Section {
    let why = "Readiness monitors are the engine's self-tests for emissions parts. Inspection stations check them, and after codes are cleared they all reset to incomplete.";
    let Some(rd) = &r.readiness else {
        return Section {
            id: "readiness",
            title: "Emissions readiness",
            grade: Grade::NotChecked,
            headline: "Readiness could not be read.".into(),
            details: vec![],
            why,
        };
    };
    let supported: Vec<_> = rd.monitors.iter().filter(|m| m.supported).collect();
    let incomplete: Vec<_> = supported.iter().filter(|m| !m.complete).collect();
    let details = incomplete
        .iter()
        .map(|m| format!("{}: not finished", m.name))
        .collect();
    let (grade, headline) = match incomplete.len() {
        0 => (
            Grade::Good,
            format!("All {} supported monitors have finished.", supported.len()),
        ),
        1 => (
            Grade::Attention,
            "One monitor has not finished. Many inspection programs allow one.".into(),
        ),
        n => (
            Grade::Problem,
            format!("{n} monitors have not finished. Many inspection programs allow at most one, so the car may not pass yet."),
        ),
    };
    Section {
        id: "readiness",
        title: "Emissions readiness",
        grade,
        headline,
        details,
        why,
    }
}

/// Below these, the codes were cleared recently.
const RECENT_KM: f64 = 160.0;
const RECENT_WARMUPS: f64 = 10.0;

fn cleared(r: &Readings) -> Section {
    let why = "Clearing codes turns the engine light off for a while, which can hide a fault from a buyer. The car keeps counting the distance and warm-ups since the last clear, and its self-tests reset, so a recent clear still shows.";
    let incomplete = r.readiness.as_ref().map(|rd| rd.incomplete());
    if r.km_since_clear.is_none() && r.warmups_since_clear.is_none() {
        return Section {
            id: "cleared",
            title: "Codes cleared recently?",
            grade: Grade::NotChecked,
            headline: "This car does not report when its codes were last cleared.".into(),
            details: vec![],
            why,
        };
    }
    let mut details = Vec::new();
    if let Some(km) = r.km_since_clear {
        details.push(format!(
            "Distance since the last clear: {} km ({} miles)",
            whole(km),
            whole(km * 0.621_371)
        ));
    }
    if let Some(w) = r.warmups_since_clear {
        details.push(format!("Engine warm-ups since the last clear: {w:.0}"));
    }
    let recent_km = r.km_since_clear.is_some_and(|k| k < RECENT_KM);
    let recent_warm = r.warmups_since_clear.is_some_and(|w| w < RECENT_WARMUPS);
    let resets = incomplete.unwrap_or(0) >= 2;
    let (grade, headline) = if (recent_km || recent_warm) && resets {
        (
            Grade::Problem,
            "Codes were cleared very recently, and the self-tests have not finished since. Ask the seller why, and re-check after a week of driving.".to_string(),
        )
    } else if recent_km || recent_warm {
        (
            Grade::Attention,
            "Codes were cleared recently. That's normal after a repair; ask what was fixed."
                .to_string(),
        )
    } else {
        (Grade::Good, "No sign of a recent clear.".to_string())
    };
    Section {
        id: "cleared",
        title: "Codes cleared recently?",
        grade,
        headline,
        details,
        why,
    }
}

fn battery(r: &Readings) -> Section {
    let why = "Weak batteries and failing alternators cause odd electrical faults on modern cars. Resting voltage shows the battery's charge; running voltage shows the alternator is charging.";
    let (Some(v), running) = (r.battery_v, r.engine_running) else {
        return Section {
            id: "battery",
            title: "Battery and charging",
            grade: Grade::NotChecked,
            headline: "Voltage could not be read.".into(),
            details: vec![],
            why,
        };
    };
    let running = running == Some(true);
    let (grade, headline) = if running {
        match v {
            v if v < 13.2 => (
                Grade::Problem,
                format!("{v:.1} V with the engine running: the battery is not being charged properly."),
            ),
            v if v < 13.5 => (
                Grade::Attention,
                format!("{v:.1} V with the engine running: charging is on the low side."),
            ),
            v if v > 15.0 => (
                Grade::Problem,
                format!("{v:.1} V with the engine running: that's higher than normal, which can damage the battery."),
            ),
            v => (
                Grade::Good,
                format!("{v:.1} V with the engine running: charging normally."),
            ),
        }
    } else {
        match v {
            v if v < 12.2 => (
                Grade::Problem,
                format!("{v:.1} V with the engine off: the battery is low."),
            ),
            v if v < 12.4 => (
                Grade::Attention,
                format!("{v:.1} V with the engine off: the battery is only partly charged."),
            ),
            v => (
                Grade::Good,
                format!("{v:.1} V with the engine off: the battery is well charged."),
            ),
        }
    };
    Section {
        id: "battery",
        title: "Battery and charging",
        grade,
        headline,
        details: vec![
            if running {
                "For a complete picture, also check the resting voltage with the engine off.".into()
            } else {
                "Start the engine and run the report again to check charging.".into()
            },
            "A voltage reading is a quick check, not a battery load test.".into(),
        ],
        why,
    }
}

fn modules(r: &Readings) -> Section {
    let why = "Every control module (brakes, airbags, body, comfort and more) keeps its own fault memory, which the engine light does not show. A car can look perfect with faults stored here.";
    if r.modules.is_empty() {
        return Section {
            id: "modules",
            title: "All modules",
            grade: Grade::NotChecked,
            headline: "No modules were scanned.".into(),
            details: vec![],
            why,
        };
    }
    let answered: Vec<_> = r.modules.iter().filter(|m| m.present).collect();
    let with_faults: Vec<_> = answered.iter().filter(|m| !m.faults.is_empty()).collect();
    let safety = with_faults.iter().any(|m| m.system == System::Safety);
    let faults: usize = with_faults.iter().map(|m| m.faults.len()).sum();
    let mut details: Vec<String> = with_faults
        .iter()
        .flat_map(|m| {
            list(&m.faults)
                .into_iter()
                .map(move |f| format!("{}: {f}", m.name))
        })
        .collect();
    if r.modules_not_scanned > 0 {
        details.push(format!(
            "{} more {} not scanned: their addresses aren't confirmed for this car yet.",
            r.modules_not_scanned,
            plural(r.modules_not_scanned, "module was", "modules were")
        ));
    }
    let (grade, headline) = if with_faults.is_empty() {
        (
            Grade::Good,
            format!("{} modules answered with no faults stored.", answered.len()),
        )
    } else if safety {
        (
            Grade::Problem,
            format!(
                "{faults} {} stored, including in a safety system. Have it checked before buying.",
                plural(faults, "fault", "faults")
            ),
        )
    } else {
        (
            Grade::Attention,
            format!(
                "{faults} {} stored in {} of {} modules.",
                plural(faults, "fault", "faults"),
                with_faults.len(),
                answered.len()
            ),
        )
    };
    Section {
        id: "modules",
        title: "All modules",
        grade,
        headline,
        details,
        why,
    }
}

fn mileage(r: &Readings) -> Section {
    let why = "A wound-back odometer hides wear. Several modules record mileage of their own, so on cars where those records are confirmed, the app compares them.";
    let mut details = Vec::new();
    if let Some(km) = r.odometer_km {
        details.push(format!(
            "Odometer reported by the engine module: {} km ({} miles)",
            whole(km),
            whole(km * 0.621_371)
        ));
    }
    if let Some(km) = r.km_with_mil.filter(|k| *k > 0.0) {
        details.push(format!("Driven with the engine light on: {} km", whole(km)));
    }
    details.push("The module-by-module mileage comparison needs mileage records confirmed for this car; it isn't available yet, so compare the dashboard with the service history.".into());
    Section {
        id: "mileage",
        title: "Mileage",
        grade: Grade::NotChecked,
        headline: match r.odometer_km {
            Some(km) => format!(
                "{} km reported over OBD-II. Not cross-checked yet.",
                whole(km)
            ),
            None => "Not cross-checked yet.".into(),
        },
        details,
        why,
    }
}

pub fn grade(r: &Readings) -> Report {
    let sections = vec![
        engine(r),
        modules(r),
        readiness(r),
        cleared(r),
        battery(r),
        mileage(r),
    ];
    let overall = sections
        .iter()
        .map(|s| s.grade)
        .max()
        .unwrap_or(Grade::NotChecked);
    let count = |g| sections.iter().filter(|s| s.grade == g).count();
    let summary = match overall {
        Grade::Good => "Everything the app could check looks good.".to_string(),
        Grade::Attention => format!(
            "{} {} worth a closer look; nothing serious found.",
            count(Grade::Attention),
            plural(count(Grade::Attention), "area is", "areas are")
        ),
        Grade::Problem => format!(
            "{} {} a problem to sort out{}.",
            count(Grade::Problem),
            plural(count(Grade::Problem), "area has", "areas have"),
            match count(Grade::Attention) {
                0 => String::new(),
                n => format!(
                    ", and {n} more {} a closer look",
                    plural(n, "needs", "need")
                ),
            }
        ),
        Grade::NotChecked => "The car could not be read.".to_string(),
    };
    Report {
        overall,
        summary,
        sections,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obd2::decode_readiness;

    fn code(c: &str) -> Code {
        Code {
            code: c.into(),
            description: None,
        }
    }

    fn healthy() -> Readings {
        Readings {
            mil_on: Some(false),
            stored: Some(vec![]),
            readiness: decode_readiness(&[0x41, 0x01, 0x00, 0x07, 0x65, 0x00]).ok(),
            km_since_clear: Some(1843.0),
            warmups_since_clear: Some(112.0),
            battery_v: Some(14.2),
            engine_running: Some(true),
            modules: vec![ModuleResult {
                name: "DSC".into(),
                system: System::Chassis,
                present: true,
                faults: vec![],
            }],
            ..Default::default()
        }
    }

    fn by<'a>(r: &'a Report, id: &str) -> &'a Section {
        r.sections.iter().find(|s| s.id == id).unwrap()
    }

    #[test]
    fn a_healthy_car_is_green_except_mileage() {
        let r = grade(&healthy());
        for s in &r.sections {
            let want = if s.id == "mileage" {
                Grade::NotChecked
            } else {
                Grade::Good
            };
            assert_eq!(s.grade, want, "{}: {}", s.id, s.headline);
        }
        assert_eq!(r.overall, Grade::Good);
    }

    #[test]
    fn spots_a_recent_clear() {
        let mut x = healthy();
        x.km_since_clear = Some(38.0);
        x.warmups_since_clear = Some(3.0);
        // Three supported monitors incomplete (catalyst, EVAP, O2).
        x.readiness = decode_readiness(&[0x41, 0x01, 0x00, 0x07, 0x65, 0x25]).ok();
        let r = grade(&x);
        assert_eq!(by(&r, "cleared").grade, Grade::Problem);
        assert_eq!(by(&r, "readiness").grade, Grade::Problem);
        assert_eq!(r.overall, Grade::Problem);
        // A recent clear with all tests finished is only worth asking about.
        x.readiness = healthy().readiness;
        assert_eq!(by(&grade(&x), "cleared").grade, Grade::Attention);
    }

    #[test]
    fn grades_codes_battery_and_safety_faults() {
        let mut x = healthy();
        x.pending = vec![code("P0456")];
        assert_eq!(by(&grade(&x), "engine").grade, Grade::Attention);
        x.stored = Some(vec![code("P0171")]);
        x.mil_on = Some(true);
        assert_eq!(by(&grade(&x), "engine").grade, Grade::Problem);
        x.battery_v = Some(12.9);
        assert_eq!(by(&grade(&x), "battery").grade, Grade::Problem);
        x.engine_running = Some(false);
        x.battery_v = Some(12.3);
        assert_eq!(by(&grade(&x), "battery").grade, Grade::Attention);
        x.modules[0].faults.push(code("5E20"));
        assert_eq!(by(&grade(&x), "modules").grade, Grade::Attention);
        x.modules[0].system = System::Safety;
        assert_eq!(by(&grade(&x), "modules").grade, Grade::Problem);
    }

    #[test]
    fn groups_thousands() {
        assert_eq!(whole(48_877.3), "48,877");
        assert_eq!(whole(999.6), "1,000");
        assert_eq!(whole(38.0), "38");
        assert_eq!(whole(1_234_567.0), "1,234,567");
    }

    #[test]
    fn unreadable_is_never_a_pass() {
        let r = grade(&Readings::default());
        assert!(r.sections.iter().all(|s| s.grade == Grade::NotChecked));
        assert_eq!(r.overall, Grade::NotChecked);
    }
}
