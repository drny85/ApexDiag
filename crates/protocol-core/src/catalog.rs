//! Every function the app can perform, with its risk tier, a plain-language
//! explanation, and the safety gates that must pass before it runs.
//!
//! The rule is fail-safe: if the app cannot *prove* a condition (for example
//! it has no speed reading yet), the gate counts as not met.

use serde::Serialize;

/// How much damage a function could do if it goes wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskTier {
    /// Reads only. Nothing in the car changes.
    Read,
    /// Runs a factory service routine or clears codes. Reversible.
    Service,
    /// Resets learned values. The car relearns them while driving.
    Adapt,
    /// Changes module settings. Backed up first and restorable.
    Code,
    /// Keys and immobilizer. A mistake can leave the car unable to start.
    Security,
    /// Rewrites module software. A failure can disable the module.
    Flash,
}

impl RiskTier {
    pub fn label(self) -> &'static str {
        match self {
            Self::Read => "Read only",
            Self::Service => "Service",
            Self::Adapt => "Adaptation",
            Self::Code => "Coding",
            Self::Security => "Keys & immobilizer",
            Self::Flash => "Programming",
        }
    }

    pub fn explain(self) -> &'static str {
        match self {
            Self::Read => "Only reads information. Nothing in the car is changed.",
            Self::Service => "Runs a factory service routine or clears stored codes. Safe when the conditions are met, and nothing permanent changes.",
            Self::Adapt => "Erases values the car has learned. It drives normally afterwards but may feel different until it relearns.",
            Self::Code => "Changes how a module behaves. The app saves the module's current settings first so they can be restored.",
            Self::Security => "Touches keys and the immobilizer. A mistake can stop the car from starting until it is put right.",
            Self::Flash => "Replaces a module's software. If it is interrupted the module can stop working, so it needs a charger and a cable connection.",
        }
    }

    /// How many confirmation steps the UI demands.
    pub fn confirmations(self) -> u8 {
        match self {
            Self::Read => 0,
            Self::Service | Self::Adapt => 2,
            Self::Code | Self::Security | Self::Flash => 3,
        }
    }
}

/// A condition that must hold before a function runs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Requirement {
    /// Vehicle speed reads exactly 0.
    Stationary,
    /// Engine speed reads exactly 0 (ignition on, engine off).
    EngineOff,
    /// Engine is running.
    EngineRunning,
    /// Battery voltage at or above this many volts.
    BatteryAtLeast { volts: f32 },
    /// The user has confirmed a battery charger is connected.
    ChargerConnected,
    /// The adapter is wired (USB / ethernet), not wireless.
    WiredAdapter,
    /// A backup of the module was taken in this session.
    BackupTaken,
    /// The request bytes for this car are verified.
    JobVerified,
}

impl Requirement {
    pub fn explain(self) -> String {
        match self {
            Self::Stationary => "The car must be stopped (speed 0).".into(),
            Self::EngineOff => "Ignition on, engine off.".into(),
            Self::EngineRunning => "The engine must be running.".into(),
            Self::BatteryAtLeast { volts } => format!("Battery at {volts:.1} V or higher."),
            Self::ChargerConnected => "A battery charger or power supply is connected.".into(),
            Self::WiredAdapter => "A wired adapter is in use. Bluetooth can drop mid-write.".into(),
            Self::BackupTaken => "The module is backed up to the Time Machine right before the write. If the backup fails, nothing is written.".into(),
            Self::JobVerified => "This routine's commands have been verified for this car.".into(),
        }
    }
}

/// What the app currently knows about the car. `None` means "unknown".
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct VehicleState {
    pub speed_kmh: Option<f64>,
    pub rpm: Option<f64>,
    pub battery_v: Option<f64>,
    pub charger_confirmed: bool,
    pub wired_adapter: bool,
    pub backup_taken: bool,
    pub job_verified: bool,
}

impl Requirement {
    /// True only when the state proves the requirement.
    pub fn is_met(self, s: &VehicleState) -> bool {
        match self {
            Self::Stationary => s.speed_kmh == Some(0.0),
            Self::EngineOff => s.rpm == Some(0.0),
            Self::EngineRunning => s.rpm.is_some_and(|r| r > 300.0),
            Self::BatteryAtLeast { volts } => s.battery_v.is_some_and(|v| v >= volts as f64),
            Self::ChargerConnected => s.charger_confirmed,
            Self::WiredAdapter => s.wired_adapter,
            Self::BackupTaken => s.backup_taken,
            Self::JobVerified => s.job_verified,
        }
    }
}

/// A user-facing function with its explanation.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Function {
    pub id: &'static str,
    pub name: &'static str,
    pub tier: RiskTier,
    /// What it does.
    pub what: &'static str,
    /// Why you'd use it.
    pub why: &'static str,
    /// What can go wrong.
    pub risks: &'static str,
    /// How to undo it.
    pub undo: &'static str,
    pub requires: &'static [Requirement],
}

impl Function {
    /// Requirements that are not met right now. Empty means it may run.
    pub fn unmet(&self, state: &VehicleState) -> Vec<Requirement> {
        self.requires
            .iter()
            .copied()
            .filter(|r| !r.is_met(state))
            .collect()
    }
}

use Requirement::*;

const BATTERY_OK: Requirement = BatteryAtLeast { volts: 12.2 };
const BATTERY_FLASH: Requirement = BatteryAtLeast { volts: 13.0 };

/// The full function catalog.
pub const FUNCTIONS: &[Function] = &[
    Function {
        id: "live_data",
        name: "Live data",
        tier: RiskTier::Read,
        what: "Streams sensor readings such as engine speed, temperatures and fuel trims.",
        why: "See how the engine is behaving right now, and confirm a repair worked.",
        risks: "None. Watching gauges while driving is distracting; let a passenger do it.",
        undo: "Nothing to undo.",
        requires: &[],
    },
    Function {
        id: "read_codes",
        name: "Read fault codes",
        tier: RiskTier::Read,
        what: "Reads stored, pending and permanent fault codes from the engine and every module the vehicle pack knows.",
        why: "Find out why a warning light is on, or check a car before buying it.",
        risks: "None.",
        undo: "Nothing to undo.",
        requires: &[],
    },
    Function {
        id: "module_scan",
        name: "Module scan",
        tier: RiskTier::Read,
        what: "Asks every address on the diagnostic bus whether a module is there.",
        why: "Shows exactly which modules this car has, so nothing is missed.",
        risks: "None. Some modules may briefly log a 'diagnosis active' entry.",
        undo: "Nothing to undo.",
        requires: &[],
    },
    Function {
        id: "clear_codes",
        name: "Clear fault codes",
        tier: RiskTier::Service,
        what: "Erases stored fault codes and, for the engine, resets emissions readiness monitors.",
        why: "After a repair, so you can see whether the fault comes back.",
        risks: "Clearing hides the evidence; read and save codes first (the app does this automatically). Readiness monitors reset, so the car will fail an emissions test until it has been driven enough.",
        undo: "Cannot be undone, but the app keeps a copy of the codes it cleared.",
        requires: &[Stationary],
    },
    Function {
        id: "coolant_bleed",
        name: "Coolant bleeding",
        tier: RiskTier::Service,
        what: "Runs the electric water pump through the factory bleed cycle to push trapped air out of the cooling system.",
        why: "Required after draining coolant or replacing the water pump, thermostat or a hose. Trapped air causes overheating.",
        risks: "Running the pump dry can damage it, so the system must be full. Coolant may overflow if the cap is off. The cycle takes about 12 minutes and draws battery power.",
        undo: "Stop it at any time; nothing is permanently changed.",
        requires: &[Stationary, EngineOff, BATTERY_OK, JobVerified],
    },
    Function {
        id: "battery_registration",
        name: "Battery registration",
        tier: RiskTier::Service,
        what: "Tells the power management system a new battery has been fitted.",
        why: "The car adjusts charging to battery age. Without registering, a new battery is overcharged like an old one and wears out early.",
        risks: "Registering without actually replacing the battery makes the car undercharge the old one.",
        undo: "Register again after the next real battery change.",
        requires: &[Stationary, EngineOff, BATTERY_OK, JobVerified],
    },
    Function {
        id: "dme_adaptation_reset",
        name: "Engine adaptation reset",
        tier: RiskTier::Adapt,
        what: "Clears the engine's learned fuel trims, idle and throttle values.",
        why: "After replacing a sensor, injector or fixing a vacuum leak, so the engine relearns from a clean start.",
        risks: "Idle may be rough and fuel economy slightly off for the first 20 to 50 km while it relearns.",
        undo: "Relearns automatically while driving.",
        requires: &[Stationary, EngineOff, BATTERY_OK, JobVerified],
    },
    Function {
        id: "egs_adaptation_reset",
        name: "Transmission adaptation reset",
        tier: RiskTier::Adapt,
        what: "Clears the automatic transmission's learned clutch pressures and shift timing.",
        why: "After a fluid change or repair, or when shifts are harsh because of stale learned values.",
        risks: "Shifts can be firm or slow until it relearns over a few drives. Do not reset on a slipping transmission without diagnosing it first.",
        undo: "Relearns automatically while driving.",
        requires: &[Stationary, EngineOff, BATTERY_OK, JobVerified],
    },
    Function {
        id: "actuator_test",
        name: "Actuator tests",
        tier: RiskTier::Service,
        what: "Switches individual parts on and off: fans, pumps, valves, lights and motors.",
        why: "Prove whether a part works before replacing it.",
        risks: "Moving parts can start suddenly. Keep hands clear of the engine bay and fans.",
        undo: "Every test stops automatically and the module returns control to normal.",
        requires: &[Stationary, BATTERY_OK, JobVerified],
    },
    Function {
        id: "coding",
        name: "Module coding",
        tier: RiskTier::Code,
        what: "Changes feature settings stored in a module, such as lights, locks and comfort options.",
        why: "Turn factory features on or off, or code a replacement module to this car.",
        risks: "Wrong values can disable features or cause warnings. If the battery runs low mid-write the module can be left half-coded.",
        undo: "Restore the backup the app takes before every write.",
        requires: &[Stationary, EngineOff, BATTERY_OK, BackupTaken, JobVerified],
    },
    Function {
        id: "snapshot_restore",
        name: "Restore module coding",
        tier: RiskTier::Code,
        what: "Writes a module's coding back to how it was in an earlier Time Machine snapshot. Only settings that differ are written, then each one is read back to check it.",
        why: "Undo a coding change that went wrong, or put back settings a shop changed.",
        risks: "Same as coding: wrong values can disable features or cause warnings, and a flat battery mid-write can leave the module half-coded. Learned values and fault memory are not restored; the car relearns those by itself.",
        undo: "The current coding is snapshotted first, so a restore can itself be undone from the Time Machine.",
        requires: &[Stationary, EngineOff, BATTERY_OK, BackupTaken, JobVerified],
    },
    Function {
        id: "key_programming",
        name: "Key programming",
        tier: RiskTier::Security,
        what: "Adds or disables a key in the car's key and immobilizer module (CAS on BMW, EZS on Mercedes-Benz, smart key ECU on Toyota and Honda), using the car's own security access.",
        why: "Add a spare key, or disable a lost one so it can no longer start the car.",
        risks: "A mistake can leave the car unable to start until it is put right. Needs the car's security code, read from the car with legitimate access. Disabled keys cannot be re-enabled without reprogramming.",
        undo: "Restore the key module backup. Disabled keys can be re-added.",
        requires: &[Stationary, EngineOff, BATTERY_FLASH, ChargerConnected, BackupTaken, JobVerified],
    },
    Function {
        id: "flash",
        name: "Module programming",
        tier: RiskTier::Flash,
        what: "Replaces a module's software with a different version.",
        why: "Install a software update that fixes a known fault.",
        risks: "If power or the connection drops mid-write, the module may stop working and need recovery or replacement. Needs a charger and a wired adapter; Bluetooth is not allowed.",
        undo: "Re-flash the saved original software.",
        requires: &[Stationary, EngineOff, BATTERY_FLASH, ChargerConnected, WiredAdapter, BackupTaken, JobVerified],
    },
];

/// Looks up a function by id.
pub fn function(id: &str) -> Option<&'static Function> {
    FUNCTIONS.iter().find(|f| f.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_state_blocks_everything_but_reads() {
        let unknown = VehicleState::default();
        for f in FUNCTIONS {
            assert_eq!(
                f.unmet(&unknown).is_empty(),
                f.tier == RiskTier::Read,
                "{}",
                f.id
            );
        }
    }

    #[test]
    fn every_write_requires_stationary() {
        for f in FUNCTIONS.iter().filter(|f| f.tier > RiskTier::Read) {
            assert!(f.requires.contains(&Stationary), "{}", f.id);
            assert!(f.tier.confirmations() >= 2, "{}", f.id);
        }
    }

    #[test]
    fn moving_car_blocks_coolant_bleed() {
        let f = function("coolant_bleed").unwrap();
        let mut s = VehicleState {
            speed_kmh: Some(0.0),
            rpm: Some(0.0),
            battery_v: Some(12.6),
            job_verified: true,
            ..Default::default()
        };
        assert!(f.unmet(&s).is_empty());
        s.speed_kmh = Some(3.0);
        assert_eq!(f.unmet(&s), vec![Stationary]);
        s.speed_kmh = Some(0.0);
        s.rpm = Some(750.0);
        assert_eq!(f.unmet(&s), vec![EngineOff]);
    }

    #[test]
    fn flash_needs_wired_adapter_and_charger() {
        let f = function("flash").unwrap();
        let s = VehicleState {
            speed_kmh: Some(0.0),
            rpm: Some(0.0),
            battery_v: Some(13.5),
            backup_taken: true,
            job_verified: true,
            ..Default::default()
        };
        assert_eq!(f.unmet(&s), vec![ChargerConnected, WiredAdapter]);
    }

    #[test]
    fn ids_unique_and_explained() {
        for (i, a) in FUNCTIONS.iter().enumerate() {
            assert!(
                !a.what.is_empty()
                    && !a.why.is_empty()
                    && !a.risks.is_empty()
                    && !a.undo.is_empty()
            );
            for b in &FUNCTIONS[i + 1..] {
                assert_ne!(a.id, b.id);
            }
        }
    }
}
