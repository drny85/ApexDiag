//! Vehicle packs: what each make and model has inside, and how to reach it.
//!
//! The protocol engine (ISO-TP, ELM327, KWP2000, UDS, OBD-II) is the same
//! for every car. What differs between brands is *where* the modules are,
//! which diagnostic language each speaks, what their fault codes mean and
//! which service routines exist. A [`Pack`] holds that for one family of
//! cars. Generic OBD-II works on any car without a pack.

use serde::Serialize;

use crate::diag::Protocol;
use crate::isotp::Addressing;
use crate::snapshot::RecordDef;
use crate::CanId;

/// BMW tester request ID on D-CAN.
pub const BMW_TESTER_ID: CanId = 0x6F1;
/// BMW tester address, the first data byte of every module reply.
pub const BMW_TESTER_ADDRESS: u8 = 0xF1;

/// What part of the car a module belongs to, for grouping in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum System {
    Powertrain,
    Chassis,
    Body,
    Comfort,
    Infotainment,
    Safety,
}

/// How a module is addressed on the diagnostic CAN bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Link {
    /// BMW D-CAN: requests go out on 0x6F1 with the module address as the
    /// first data byte; replies come back on 0x600 + address.
    BmwExtended,
    /// Plain 11-bit ISO-TP with a fixed request and response ID.
    Can11 { tx: u16, rx: u16 },
}

/// Where a module sits in the car, on the top-down map (viewBox 240 × 470,
/// front at the top, driver on the left). Approximate by design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Spot {
    pub x: u16,
    pub y: u16,
    pub area: &'static str,
}

/// One control module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Module {
    /// Short name as the brand's own tools show it (DME, ME, ECM, PGM-FI).
    pub name: &'static str,
    /// What it is, in plain words.
    pub description: &'static str,
    /// What it controls and why you'd talk to it.
    pub role: &'static str,
    /// Key that is unique within the pack. For BMW it is the real module
    /// address; for 11-bit cars the low byte of the request ID.
    pub address: u8,
    pub system: System,
    pub link: Link,
    pub protocol: Protocol,
    pub spot: Spot,
}

impl Module {
    /// CAN ID the tester sends on.
    pub const fn request_id(&self) -> CanId {
        match self.link {
            Link::BmwExtended => BMW_TESTER_ID,
            Link::Can11 { tx, .. } => tx as CanId,
        }
    }

    /// CAN ID the module answers on.
    pub const fn response_id(&self) -> CanId {
        match self.link {
            Link::BmwExtended => 0x600 + self.address as CanId,
            Link::Can11 { rx, .. } => rx as CanId,
        }
    }

    /// ISO-TP addressing of the module's replies.
    pub const fn reply_addressing(&self) -> Addressing {
        match self.link {
            Link::BmwExtended => Addressing::Extended(BMW_TESTER_ADDRESS),
            Link::Can11 { .. } => Addressing::Normal,
        }
    }

    pub const fn protocol(&self) -> Protocol {
        self.protocol
    }

    /// The engine and transmission IDs every OBD-II car must answer on.
    pub const fn is_obd_legislated(&self) -> bool {
        matches!(self.link, Link::Can11 { tx, .. } if tx >= 0x7E0 && tx <= 0x7E7)
    }
}

/// Status of a special-function definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Payload confirmed against a real car.
    Verified,
    /// We know what the job does but not yet the exact request bytes for
    /// this car. The app refuses to run it until it is verified.
    PendingCapture,
}

/// A service job: which module, what it sends, and how sure we are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Job {
    /// Catalog id, matches [`crate::catalog`].
    pub function_id: &'static str,
    /// The brand's own job name, where we are sure of it.
    pub oem_job: Option<&'static str>,
    /// Address of the module it runs in.
    pub module: u8,
    /// Request bytes, if known.
    pub request: Option<&'static [u8]>,
    pub status: JobStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    Bmw,
    MercedesBenz,
    Toyota,
    Honda,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyStyle {
    Suv,
    Sedan,
}

/// How sure we are of a pack's module map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// Addresses widely documented for these cars: scanned on a real car.
    Documented,
    /// A demo layout. On a real car only the legislated OBD-II engine and
    /// transmission IDs are scanned until each module is verified.
    Demo,
}

/// Everything the app knows about one family of cars.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Pack {
    pub id: &'static str,
    pub make: Make,
    /// Brand as written by the maker.
    pub brand: &'static str,
    pub model: &'static str,
    /// Chassis or generation code (E70, W167, XV70).
    pub chassis: &'static str,
    pub years: &'static str,
    pub body: BodyStyle,
    pub layout: Layout,
    /// What the brand's own dealer tool is called, for orientation.
    pub dealer_tool: &'static str,
    pub modules: &'static [Module],
    pub records: &'static [RecordDef],
    pub jobs: &'static [Job],
    /// Catalog function ids this pack offers.
    pub functions: &'static [&'static str],
    /// Brand-specific notes shown to the user.
    pub notes: &'static [&'static str],
}

impl Pack {
    pub fn module(&self, address: u8) -> Option<&'static Module> {
        self.modules.iter().find(|m| m.address == address)
    }

    pub fn records_for(&self, address: u8) -> impl Iterator<Item = &'static RecordDef> {
        self.records.iter().filter(move |r| r.module == address)
    }

    pub fn offers(&self, function_id: &str) -> bool {
        self.functions.contains(&function_id)
    }

    /// Modules to scan on a real car: all of a documented layout, only the
    /// legislated OBD IDs of a demo layout.
    pub fn scannable_on_real_car(&self) -> impl Iterator<Item = &'static Module> {
        let all = self.layout == Layout::Documented;
        self.modules
            .iter()
            .filter(move |m| all || m.is_obd_legislated())
    }
}

/// Any car: generic OBD-II only.
pub const GENERIC: Pack = Pack {
    id: "generic",
    make: Make::Generic,
    brand: "Any make",
    model: "OBD-II vehicle",
    chassis: "OBD-II",
    years: "1996 on (US), 2001 on (EU petrol)",
    body: BodyStyle::Sedan,
    layout: Layout::Demo,
    dealer_tool: "Generic OBD-II",
    modules: &[],
    records: &[],
    jobs: &[],
    functions: &["clear_codes"],
    notes: &["Live data, engine fault codes, emissions readiness, freeze frame and VIN work on any OBD-II car. Deeper module access needs a vehicle pack for the brand."],
};

/// Every pack, most specific first.
pub fn packs() -> [&'static Pack; 5] {
    [
        &crate::bmw_e70::PACK,
        &crate::packs::mercedes_w167::PACK,
        &crate::packs::toyota_xv70::PACK,
        &crate::packs::honda_accord::PACK,
        &GENERIC,
    ]
}

pub fn pack(id: &str) -> Option<&'static Pack> {
    packs().into_iter().find(|p| p.id == id)
}

/// The pack for a make, or the generic one.
pub fn pack_for_make(make: Make) -> &'static Pack {
    packs()
        .into_iter()
        .find(|p| p.make == make)
        .unwrap_or(&GENERIC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_keys_are_unique_and_ids_consistent() {
        for p in packs() {
            for (i, a) in p.modules.iter().enumerate() {
                for b in &p.modules[i + 1..] {
                    assert_ne!(a.address, b.address, "{}: {} / {}", p.id, a.name, b.name);
                    assert_ne!(
                        a.response_id(),
                        b.response_id(),
                        "{}: {} / {}",
                        p.id,
                        a.name,
                        b.name
                    );
                }
                if let Link::Can11 { tx, rx } = a.link {
                    assert_eq!(a.address, (tx & 0xFF) as u8, "{}: {}", p.id, a.name);
                    assert_ne!(tx, rx);
                }
                assert!(a.spot.x <= 240 && a.spot.y <= 470, "{}: {}", p.id, a.name);
            }
        }
    }

    #[test]
    fn jobs_records_and_functions_point_at_real_things() {
        for p in packs() {
            for j in p.jobs {
                assert!(p.module(j.module).is_some(), "{}: {}", p.id, j.function_id);
                assert!(crate::catalog::function(j.function_id).is_some());
                assert_eq!(j.status == JobStatus::Verified, j.request.is_some());
            }
            for r in p.records {
                assert!(p.module(r.module).is_some(), "{}: {}", p.id, r.name);
            }
            for f in p.functions {
                assert!(crate::catalog::function(f).is_some(), "{}: {f}", p.id);
            }
            for (i, a) in p.records.iter().enumerate() {
                for b in &p.records[i + 1..] {
                    assert!((a.module, a.id) != (b.module, b.id), "{}: {}", p.id, a.name);
                }
            }
        }
    }

    #[test]
    fn demo_layouts_scan_only_obd_ids_on_a_real_car() {
        let toyota = pack_for_make(Make::Toyota);
        assert!(toyota
            .scannable_on_real_car()
            .all(|m| m.is_obd_legislated()));
        assert!(toyota.scannable_on_real_car().count() >= 1);
        let bmw = pack_for_make(Make::Bmw);
        assert_eq!(bmw.scannable_on_real_car().count(), bmw.modules.len());
        assert_eq!(pack_for_make(Make::Generic).id, "generic");
    }
}
