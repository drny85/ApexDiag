//! BMW E70 X5 (2007-2013) module address map for KWP2000 over D-CAN.
//!
//! The tester sends on CAN ID 0x6F1 with the target ECU address as the first
//! data byte (extended addressing). The ECU replies on 0x600 + its address.
//! Step 4 adds job definitions and special-function payloads.

use crate::CanId;

/// Tester request CAN ID used for all physical requests.
pub const TESTER_ID: CanId = 0x6F1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module {
    pub name: &'static str,
    pub description: &'static str,
    pub address: u8,
}

impl Module {
    /// CAN ID the module answers on.
    pub const fn response_id(&self) -> CanId {
        0x600 + self.address as CanId
    }
}

pub const DME: Module = Module {
    name: "DME",
    description: "Engine (petrol)",
    address: 0x12,
};
pub const EGS: Module = Module {
    name: "EGS",
    description: "Transmission",
    address: 0x18,
};
pub const DSC: Module = Module {
    name: "DSC",
    description: "ABS / stability control",
    address: 0x29,
};
pub const CAS: Module = Module {
    name: "CAS",
    description: "Car access / immobilizer",
    address: 0x40,
};
pub const JBE: Module = Module {
    name: "JBE",
    description: "Junction box",
    address: 0x00,
};

pub const MODULES: &[Module] = &[DME, EGS, DSC, CAS, JBE];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_ids() {
        assert_eq!(DME.response_id(), 0x612);
        assert_eq!(EGS.response_id(), 0x618);
        assert_eq!(DSC.response_id(), 0x629);
    }
}
