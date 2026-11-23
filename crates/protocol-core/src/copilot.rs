//! Repair Copilot: a guided test plan for a fault code, measured checks the
//! app can run from live data, and a way to prove the fix worked.
//!
//! Playbooks cover generic SAE codes (P0xxx), whose meanings are the same
//! on every car, and use general workshop practice: they never claim to
//! know a maker's own test procedure. Codes without a playbook get a
//! general plan built from the code's letter (powertrain, chassis, body,
//! network).
//!
//! Everything here is pure: sampling live data happens in the app layer,
//! which passes the samples to [`evaluate`].

use serde::{Deserialize, Serialize};

/// How often a cause turns out to be the culprit, in general practice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Likelihood {
    Common,
    Possible,
    Rare,
}

/// Who can reasonably do a fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    /// Basic tools, little risk.
    Diy,
    /// Needs experience, more tools or care.
    Experienced,
    /// Best left to a shop: special tools, lifting, or safety risk.
    Shop,
}

/// The usual fix for a cause, with rough effort and cost.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Fix {
    pub action: &'static str,
    pub difficulty: Difficulty,
    /// Rough US cost range, parts and shop labour, in dollars. Varies a
    /// lot by car, brand and shop: shown as an estimate only.
    pub cost_usd: (u16, u16),
    pub time: &'static str,
    pub parts: &'static str,
    pub tools: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Cause {
    pub name: &'static str,
    pub likelihood: Likelihood,
    /// Why it causes this code, and what points to it.
    pub why: &'static str,
    pub fix: Fix,
}

/// A measurement the app runs from live data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    /// Short + long-term fuel trims per bank, engine idling.
    FuelTrims,
    /// Intake manifold pressure at warm idle.
    IntakeVacuum,
    /// Idle speed steadiness (misfires make it shake).
    IdleStability,
    /// Upstream versus downstream oxygen sensor activity.
    CatalystOxygenSensors,
    /// Coolant temperature after the engine has had time to warm up.
    WarmUp,
    /// System voltage: battery at rest or alternator output when running.
    Charging,
}

impl Check {
    /// PIDs to sample, and for how long.
    pub fn pids(self) -> &'static [u8] {
        match self {
            Check::FuelTrims => &[0x0C, 0x05, 0x06, 0x07, 0x08, 0x09],
            Check::IntakeVacuum => &[0x0C, 0x05, 0x0B],
            Check::IdleStability => &[0x0C, 0x05],
            Check::CatalystOxygenSensors => &[0x0C, 0x14, 0x15],
            Check::WarmUp => &[0x0C, 0x05, 0x1F],
            Check::Charging => &[0x0C, 0x42],
        }
    }

    pub fn sample_ms(self) -> u64 {
        match self {
            Check::CatalystOxygenSensors | Check::IdleStability => 4000,
            _ => 2500,
        }
    }

    /// Whether the engine has to be running for the result to mean anything.
    pub fn needs_engine_running(self) -> bool {
        !matches!(self, Check::Charging)
    }

    pub fn title(self) -> &'static str {
        match self {
            Check::FuelTrims => "Measure fuel trims",
            Check::IntakeVacuum => "Measure intake vacuum",
            Check::IdleStability => "Measure idle steadiness",
            Check::CatalystOxygenSensors => "Compare the oxygen sensors",
            Check::WarmUp => "Check the warm-up",
            Check::Charging => "Check battery and charging",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepKind {
    /// The app measures it.
    Auto { check: Check },
    /// The technician does it and records what they found.
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Step {
    pub id: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub kind: StepKind,
    /// Tools or parts needed, if any.
    pub tools: Option<&'static str>,
    /// What can go wrong doing this step.
    pub caution: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Playbook {
    pub id: &'static str,
    pub codes: &'static [&'static str],
    pub title: &'static str,
    /// What the code means, in plain words.
    pub meaning: &'static str,
    pub causes: &'static [Cause],
    pub steps: &'static [Step],
    /// How to prove the repair worked.
    pub verify: &'static str,
    /// Checks to re-run after the repair.
    pub verify_checks: &'static [Check],
}

const fn auto(id: &'static str, check: Check, detail: &'static str) -> Step {
    Step {
        id,
        title: check_title(check),
        detail,
        kind: StepKind::Auto { check },
        tools: None,
        caution: None,
    }
}

const fn check_title(check: Check) -> &'static str {
    match check {
        Check::FuelTrims => "Measure fuel trims",
        Check::IntakeVacuum => "Measure intake vacuum",
        Check::IdleStability => "Measure idle steadiness",
        Check::CatalystOxygenSensors => "Compare the oxygen sensors",
        Check::WarmUp => "Check the warm-up",
        Check::Charging => "Check battery and charging",
    }
}

const fn manual(
    id: &'static str,
    title: &'static str,
    detail: &'static str,
    tools: Option<&'static str>,
    caution: Option<&'static str>,
) -> Step {
    Step {
        id,
        title,
        detail,
        kind: StepKind::Manual,
        tools,
        caution,
    }
}

const fn cause(name: &'static str, likelihood: Likelihood, why: &'static str, fix: Fix) -> Cause {
    Cause {
        name,
        likelihood,
        why,
        fix,
    }
}

const fn fix(
    action: &'static str,
    difficulty: Difficulty,
    cost_usd: (u16, u16),
    time: &'static str,
    parts: &'static str,
    tools: &'static str,
) -> Fix {
    Fix {
        action,
        difficulty,
        cost_usd,
        time,
        parts,
        tools,
    }
}

use Difficulty::*;
use Likelihood::*;

const HOT_ENGINE: &str = "The engine bay is hot and the fan can start on its own. Keep hands, ties and sleeves clear of belts and the fan.";

pub const PLAYBOOKS: &[Playbook] = &[
    Playbook {
        id: "lean",
        codes: &["P0171", "P0174"],
        title: "Engine running lean",
        meaning: "The engine computer has added as much fuel as it is allowed and the oxygen sensors still see too much air. Either extra air is getting in that the air-flow sensor never measured, or not enough fuel is arriving.",
        causes: &[
            cause("Unmetered air (vacuum or intake leak)", Common, "Air that enters after the air-flow sensor: a split hose, a leaking intake gasket, or a failed crankcase vent (PCV) valve. Trims are worst at idle and improve at higher rpm.", fix("Find the leak with a smoke test, then replace the split hose, gasket or crankcase vent valve", Experienced, (50, 450), "1-3 h", "Hose, gasket or crankcase vent valve", "Smoke machine, basic hand tools")),
            cause("Dirty or failing air-flow sensor", Common, "Under-reads the air coming in, so the engine injects too little. Trims stay high across the rpm range.", fix("Clean the sensor; replace it if trims stay high", Diy, (10, 350), "15-30 min", "Air-flow sensor cleaner, or a new sensor", "Screwdriver or Torx set")),
            cause("Low fuel pressure", Possible, "A weak pump, clogged filter or faulty regulator. Trims get worse under load.", fix("Confirm with a gauge, then replace the fuel filter or pump", Shop, (150, 1100), "1-4 h", "Fuel filter or fuel pump", "Fuel pressure gauge, line tools")),
            cause("Exhaust leak before the front oxygen sensor", Possible, "Pulls outside air past the sensor, which then reads lean.", fix("Replace the leaking gasket or repair the joint", Experienced, (80, 500), "1-3 h", "Exhaust gasket or flange hardware", "Penetrating oil, sockets, lift or stands")),
            cause("Restricted injectors", Rare, "Usually shows on one bank or cylinder first, often with a misfire.", fix("Clean or replace the injector", Shop, (150, 900), "1-3 h", "Injector or injector service", "Injector tester or shop service")),
        ],
        steps: &[
            auto("trims", Check::FuelTrims, "Reads short and long-term trims on each bank at idle. Both banks lean together points to a shared cause; one bank points to that side of the engine."),
            auto("vacuum", Check::IntakeVacuum, "A healthy engine pulls strong vacuum at idle. Weak vacuum backs up a leak."),
            manual("listen", "Look and listen for leaks", "With the engine idling, listen around the intake, hoses and crankcase vent for hissing. Check the oil filler cap seal and the dipstick seal. A smoke test finds leaks you cannot hear.", Some("Smoke machine (best), or a careful ear"), Some(HOT_ENGINE)),
            manual("maf", "Inspect the air filter and air-flow sensor", "A collapsed or oily air filter, or a dirty sensor element, both cause lean trims. Clean the sensor only with spray made for air-flow sensors, never touch the element.", Some("Air-flow sensor cleaner"), Some("Disconnect the sensor only with the ignition off.")),
            manual("fuel", "Test fuel pressure", "Compare fuel pressure at idle and under load with the maker's specification. Pressure that sags under load points to the pump or filter.", Some("Fuel pressure gauge"), Some("Fuel under pressure: relieve it first, keep a fire extinguisher close and never work near sparks.")),
        ],
        verify: "Fix the cause, clear the codes, and let the engine idle and drive normally. Total trim (short + long) should settle within about ±5% and the code should not return.",
        verify_checks: &[Check::FuelTrims],
    },
    Playbook {
        id: "rich",
        codes: &["P0172", "P0175"],
        title: "Engine running rich",
        meaning: "The engine computer has removed as much fuel as it is allowed and the oxygen sensors still see too little air.",
        causes: &[
            cause("Leaking injector", Common, "Drips fuel into a cylinder; often with a misfire or fuel smell.", fix("Replace the leaking injector and change the oil if fuel got in", Shop, (200, 900), "1-3 h", "Injector, seals, engine oil", "Fuel line tools")),
            cause("Air-flow sensor over-reading", Possible, "Reports more air than is there, so too much fuel goes in.", fix("Replace the air-flow sensor", Diy, (80, 350), "15-30 min", "Air-flow sensor", "Screwdriver or Torx set")),
            cause("High fuel pressure", Possible, "A stuck regulator pushes extra fuel through every injector.", fix("Confirm with a gauge, then replace the pressure regulator", Shop, (150, 500), "1-2 h", "Fuel pressure regulator", "Fuel pressure gauge")),
            cause("Fuel in the oil or a saturated charcoal canister", Possible, "Fuel vapour arrives through the crankcase vent or purge valve.", fix("Change the oil; replace the purge valve or canister if saturated", Experienced, (60, 450), "30 min-2 h", "Oil and filter, purge valve or canister", "Basic hand tools")),
        ],
        steps: &[
            auto("trims", Check::FuelTrims, "Reads short and long-term trims on each bank at idle."),
            manual("smell", "Check the oil for fuel", "A strong fuel smell on the dipstick means fuel is getting into the oil, usually from a leaking injector.", None, Some(HOT_ENGINE)),
            manual("purge", "Check the purge valve", "A purge valve stuck open feeds fuel vapour into the intake at idle.", None, None),
        ],
        verify: "Clear the codes and drive normally. Total trim should settle within about ±5%.",
        verify_checks: &[Check::FuelTrims],
    },
    Playbook {
        id: "misfire",
        codes: &["P0300", "P0301", "P0302", "P0303", "P0304", "P0305", "P0306", "P0307", "P0308"],
        title: "Misfire",
        meaning: "One or more cylinders did not burn their fuel properly. The last digit is the cylinder (P0300 means random or several). Unburnt fuel overheats the catalytic converter, so a flashing check-engine light means stop driving.",
        causes: &[
            cause("Ignition coil", Common, "Coils crack with heat. A misfire that moves when you swap coils is the coil.", fix("Replace the coil (many owners replace all of them at once)", Diy, (40, 400), "15-45 min", "Ignition coil", "Socket set")),
            cause("Spark plug", Common, "Worn or fouled plugs misfire under load first.", fix("Replace the spark plugs as a set", Diy, (40, 350), "30 min-2 h", "Spark plugs", "Spark plug socket, torque wrench")),
            cause("Injector", Possible, "A clogged or leaking injector starves or floods one cylinder.", fix("Clean or replace the injector", Shop, (150, 900), "1-3 h", "Injector and seals", "Fuel line tools")),
            cause("Vacuum leak", Possible, "Random misfires at idle with lean trims.", fix("Smoke-test and replace the leaking hose or gasket", Experienced, (50, 450), "1-3 h", "Hose or gasket", "Smoke machine")),
            cause("Low compression", Rare, "A valve or ring problem. Confirmed with a compression test.", fix("Diagnose further (leak-down test); likely engine repair", Shop, (300, 4000), "Varies", "Depends on the finding", "Compression and leak-down testers")),
        ],
        steps: &[
            auto("idle", Check::IdleStability, "Watches engine speed at idle. A misfiring engine hunts and shakes."),
            auto("trims", Check::FuelTrims, "Lean trims alongside random misfires point to a vacuum leak rather than ignition."),
            manual("swap", "Swap the coil to another cylinder", "Move the coil from the misfiring cylinder to its neighbour, clear codes and drive. If the misfire follows the coil, replace the coil. If it stays, check the plug and injector.", None, Some("Ignition off. Coils carry very high voltage when the engine runs.")),
            manual("plugs", "Inspect the spark plugs", "Compare the plug from the misfiring cylinder with the others: oily, wet or eroded plugs tell their own story.", Some("Spark plug socket, torque wrench"), Some("Let the engine cool before removing plugs from an aluminium head.")),
            manual("compression", "Compression test", "If coil and plug are fine, test compression. All cylinders should be within about 10% of each other.", Some("Compression gauge"), None),
        ],
        verify: "Clear the codes and drive in the conditions shown in the freeze frame. The misfire counter should stay at zero and the code should not return.",
        verify_checks: &[Check::IdleStability],
    },
    Playbook {
        id: "catalyst",
        codes: &["P0420", "P0430"],
        title: "Catalytic converter efficiency low",
        meaning: "The downstream oxygen sensor is copying the upstream one, which means the catalytic converter is no longer storing oxygen the way it should.",
        causes: &[
            cause("Worn catalytic converter", Common, "Catalysts age, and misfires or oil burning speed that up.", fix("Replace the converter (fix any misfire or oil burning first)", Shop, (400, 2500), "1-3 h", "Catalytic converter, gaskets", "Lift, exhaust tools")),
            cause("Downstream oxygen sensor", Possible, "A lazy or contaminated sensor can make a good converter look bad.", fix("Replace the rear oxygen sensor", Diy, (60, 300), "20-45 min", "Oxygen sensor", "Oxygen sensor socket")),
            cause("Exhaust leak near the sensors", Possible, "Outside air confuses the readings.", fix("Replace the gasket or repair the joint", Experienced, (80, 500), "1-3 h", "Exhaust gasket", "Sockets, lift or stands")),
            cause("Engine problem damaging the converter", Possible, "Rich running or misfires overheat it. Fix those first or a new converter fails too.", fix("Fix the misfire, rich running or oil burning first", Experienced, (50, 1500), "Varies", "Depends on the cause", "As for that repair")),
        ],
        steps: &[
            auto("trims", Check::FuelTrims, "Rules out a fuel problem first. A rich or lean engine poisons a new converter."),
            auto("o2", Check::CatalystOxygenSensors, "A working converter makes the downstream sensor hold steady while the upstream one switches. If both switch, the converter is not storing oxygen."),
            manual("leaks", "Check for exhaust leaks", "With the engine cold, look for soot marks at joints and flanges between the engine and the rear sensor.", None, Some("Exhaust parts get very hot within minutes of starting.")),
        ],
        verify: "After the repair, clear the codes and complete a drive cycle (mixed town and steady highway driving). The catalyst readiness monitor must show complete with no code.",
        verify_checks: &[Check::CatalystOxygenSensors],
    },
    Playbook {
        id: "thermostat",
        codes: &["P0128"],
        title: "Engine not reaching temperature",
        meaning: "The engine took too long to warm up, or never got there. Almost always the thermostat stuck open.",
        causes: &[
            cause("Thermostat stuck open", Common, "Coolant goes through the radiator from cold, so the engine never gets warm in cool weather.", fix("Replace the thermostat and bleed the cooling system", Experienced, (120, 600), "1-3 h", "Thermostat, gasket, coolant", "Drain pan, hand tools")),
            cause("Coolant temperature sensor", Possible, "Reads lower than the real temperature.", fix("Replace the sensor", Diy, (40, 200), "20-45 min", "Coolant temperature sensor", "Hand tools, some coolant")),
            cause("Low coolant", Possible, "An air pocket around the sensor gives a false low reading.", fix("Top up, then find and fix the leak", Diy, (15, 600), "15 min-3 h", "Coolant, plus the leaking part", "Pressure tester (for the leak)")),
        ],
        steps: &[
            auto("warmup", Check::WarmUp, "After ten minutes of running a healthy engine is above about 80 °C."),
            manual("level", "Check the coolant level", "Only with the engine cold. Low coolant means a leak to find as well.", None, Some("Never open a hot cooling system: it is under pressure and can scald.")),
        ],
        verify: "After replacing the thermostat, bleed the cooling system, then check the engine reaches normal temperature within ten minutes of driving.",
        verify_checks: &[Check::WarmUp],
    },
    Playbook {
        id: "evap",
        codes: &["P0440", "P0442", "P0455", "P0456"],
        title: "Fuel vapour (EVAP) leak",
        meaning: "The system that stores fuel tank vapour failed its leak test. It does not affect how the car drives but it fails emissions.",
        causes: &[
            cause("Loose or worn fuel cap", Common, "The cheapest fix: check the seal and click it tight.", fix("Tighten or replace the fuel cap", Diy, (15, 40), "5 min", "Fuel cap", "None")),
            cause("Purge or vent valve", Possible, "A valve that does not seal fails the test.", fix("Replace the valve", Experienced, (60, 350), "30 min-1.5 h", "Purge or vent valve", "Hand tools")),
            cause("Cracked hose or canister", Possible, "Found with a smoke test.", fix("Replace the hose or canister", Experienced, (60, 600), "1-2 h", "EVAP hose or canister", "Smoke machine, hand tools")),
        ],
        steps: &[
            manual("cap", "Check the fuel cap", "Inspect the seal for cracks and tighten until it clicks. A cap fix takes several drives to show as fixed.", None, None),
            manual("smoke", "Smoke-test the EVAP system", "A smoke machine through the service port shows the leak.", Some("EVAP smoke machine"), Some("Use only an EVAP-rated smoke machine with inert gas or no flame near the fuel system.")),
        ],
        verify: "Clear the codes. The EVAP monitor can take several days of driving to run: it needs a cold start and a tank between about a quarter and three quarters full.",
        verify_checks: &[],
    },
    Playbook {
        id: "cam_crank",
        codes: &["P0016", "P0017", "P0018", "P0019"],
        title: "Camshaft and crankshaft out of step",
        meaning: "The engine computer sees the camshaft and crankshaft out of their expected relationship. It can be a sensor, the variable valve timing, or the timing chain itself.",
        causes: &[
            cause("Low or dirty engine oil", Common, "Variable valve timing is moved by oil pressure. Check this first: it is free.", fix("Top up, or change the oil and filter", Diy, (40, 150), "30 min", "Correct oil and filter", "Drain pan, filter wrench")),
            cause("Valve timing solenoid or actuator", Possible, "A sticking solenoid or worn actuator leaves the camshaft in the wrong place.", fix("Clean the screen or replace the solenoid", Experienced, (80, 600), "30 min-2 h", "Valve timing solenoid", "Hand tools")),
            cause("Camshaft or crankshaft sensor", Possible, "A weak signal looks like a timing error.", fix("Replace the sensor", Diy, (40, 300), "20 min-1 h", "Position sensor", "Hand tools")),
            cause("Stretched timing chain", Possible, "Common on high-mileage engines. Needs a proper timing check before parts are ordered.", fix("Replace the timing chain, guides and tensioner", Shop, (1200, 4500), "8-20 h", "Timing chain kit", "Engine timing tools")),
        ],
        steps: &[
            manual("oil", "Check oil level and condition", "Low or sludgy oil is the first suspect. Correct it and re-test before anything else.", None, Some("Check on level ground with the engine off for a few minutes.")),
            auto("idle", Check::IdleStability, "Timing problems often make the idle rough."),
            manual("solenoid", "Inspect the valve timing solenoids", "Look for a clogged screen or damaged wiring.", None, None),
            manual("timing", "Verify mechanical timing", "If the code persists, check the timing against the maker's procedure before replacing parts.", Some("Engine-specific timing tools"), Some("Turning an engine with incorrect timing can bend valves. Follow the maker's procedure exactly.")),
        ],
        verify: "Clear the codes and drive. The code should not return after several cold starts.",
        verify_checks: &[Check::IdleStability],
    },
    Playbook {
        id: "maf",
        codes: &["P0100", "P0101", "P0102", "P0103"],
        title: "Air-flow sensor reading wrong",
        meaning: "The air-flow sensor's reading does not match what the engine expects for its speed and throttle.",
        causes: &[
            cause("Dirty sensor element", Common, "Oil or dust on the hot-wire element makes it under-read.", fix("Clean the sensor; replace it if the code returns", Diy, (10, 350), "15-30 min", "Air-flow sensor cleaner, or a new sensor", "Screwdriver or Torx set")),
            cause("Air leak after the sensor", Possible, "The sensor reads correctly but some air bypasses it.", fix("Find and fix the leak in the intake duct", Diy, (20, 200), "30 min-1 h", "Intake boot or clamp", "Screwdriver")),
            cause("Wiring or connector", Possible, "Corroded pins give an erratic signal.", fix("Clean or repair the connector and wiring", Experienced, (20, 300), "30 min-2 h", "Connector repair kit", "Multimeter, contact cleaner")),
        ],
        steps: &[
            auto("trims", Check::FuelTrims, "A faulty air-flow sensor usually drags fuel trims away from zero."),
            manual("inspect", "Inspect and clean the sensor", "Remove it with the ignition off and clean with air-flow sensor spray only.", Some("Air-flow sensor cleaner"), None),
        ],
        verify: "Clear the codes and drive. Fuel trims should stay within about ±5%.",
        verify_checks: &[Check::FuelTrims],
    },
];

/// General plans by the code's first letter, for codes without a playbook.
const GENERAL_STEPS: &[Step] = &[
    auto("charging", Check::Charging, "Low voltage causes faults in many modules at once. Rule it out first."),
    manual_simple("freeze", "Read the freeze frame and history", "When did it set, and is it active now or stored history? A history entry with nothing failing now may be an old event."),
    manual("connectors", "Check connectors and grounds", "Look for corrosion, loose pins and damaged wiring at the module and any sensor named in the fault.", None, Some("Disconnect modules only with the ignition off.")),
    manual_simple("service_info", "Look up the maker's information", "Search the code with the car's make and model. Look for service bulletins and the maker's test plan."),
];

const fn manual_simple(id: &'static str, title: &'static str, detail: &'static str) -> Step {
    manual(id, title, detail, None, None)
}

/// Finds the playbook for a code (UDS failure types like `-64` are ignored).
pub fn playbook_for(code: &str) -> Option<&'static Playbook> {
    let base = code.get(..5)?;
    PLAYBOOKS.iter().find(|p| p.codes.contains(&base))
}

/// A general plan for any code without a playbook.
pub fn general_plan(code: &str) -> Playbook {
    let (title, meaning) = match code.chars().next() {
        Some('C') => ("Chassis fault", "A chassis system (brakes, stability control, steering or suspension) logged a fault. Treat brake and steering faults as safety-critical."),
        Some('B') => ("Body fault", "A body system (lights, locks, airbags, comfort) logged a fault."),
        Some('U') => ("Network fault", "A module lost contact with another, or received data it did not trust. Often a symptom of low voltage, a loose connector or a module that was asleep."),
        _ => ("Engine or transmission fault", "A powertrain module logged a fault."),
    };
    Playbook {
        id: "general",
        codes: &[],
        title,
        meaning,
        causes: &[],
        steps: GENERAL_STEPS,
        verify: "After the repair, clear the code, run the system, and read the module again. The code should not return.",
        verify_checks: &[Check::Charging],
    }
}

/// The plan for a code: its playbook, or a general plan.
pub fn plan_for(code: &str) -> Playbook {
    playbook_for(code)
        .copied()
        .unwrap_or_else(|| general_plan(code))
}

// ---- evaluation ----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Warn,
    Fail,
    /// Could not judge (engine not running, values unsupported).
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub check: Check,
    pub verdict: Verdict,
    pub headline: String,
    pub detail: String,
    pub readings: Vec<Reading>,
}

/// Samples per PID, in the PID's unit.
pub type Samples = std::collections::BTreeMap<u8, Vec<f64>>;

fn mean(v: &[f64]) -> Option<f64> {
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

fn spread(v: &[f64]) -> Option<f64> {
    let m = mean(v)?;
    Some((v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt())
}

fn avg(s: &Samples, pid: u8) -> Option<f64> {
    s.get(&pid).and_then(|v| mean(v))
}

fn reading(name: &str, value: f64, unit: &str) -> Reading {
    Reading {
        name: name.into(),
        value: (value * 100.0).round() / 100.0,
        unit: unit.into(),
    }
}

fn result(
    check: Check,
    verdict: Verdict,
    headline: impl Into<String>,
    detail: impl Into<String>,
    readings: Vec<Reading>,
) -> CheckResult {
    CheckResult {
        check,
        verdict,
        headline: headline.into(),
        detail: detail.into(),
        readings,
    }
}

/// Judges a check from its samples. Thresholds are general rules of thumb,
/// shown to the user with the numbers so they can judge too.
pub fn evaluate(check: Check, s: &Samples) -> CheckResult {
    let rpm = avg(s, 0x0C);
    let running = rpm.is_some_and(|r| r > 300.0);
    if check.needs_engine_running() && !running {
        return result(
            check,
            Verdict::Unknown,
            "Start the engine first",
            "This check needs the engine idling, warm if possible, in Park or neutral.",
            vec![],
        );
    }
    match check {
        Check::FuelTrims => {
            let bank = |st: u8, lt: u8| Some((avg(s, st)?, avg(s, lt)?));
            let Some((st1, lt1)) = bank(0x06, 0x07) else {
                return result(
                    check,
                    Verdict::Unknown,
                    "Fuel trims not available",
                    "The engine did not report fuel trims.",
                    vec![],
                );
            };
            let b2 = bank(0x08, 0x09);
            let mut readings = vec![
                reading("Short-term B1", st1, "%"),
                reading("Long-term B1", lt1, "%"),
                reading("Total B1", st1 + lt1, "%"),
            ];
            let t1 = st1 + lt1;
            let t2 = b2.map(|(st, lt)| {
                readings.extend([
                    reading("Short-term B2", st, "%"),
                    reading("Long-term B2", lt, "%"),
                    reading("Total B2", st + lt, "%"),
                ]);
                st + lt
            });
            let worst = t2.map_or(t1, |t2| if t2.abs() > t1.abs() { t2 } else { t1 });
            let lean = worst > 0.0;
            let side = if lean {
                "lean (adding fuel)"
            } else {
                "rich (removing fuel)"
            };
            let pattern = match t2 {
                Some(t2) if (t1 - t2).abs() < 5.0 => " Both banks agree, so look for a cause they share: the intake, the air-flow sensor, the crankcase vent or fuel pressure.",
                Some(_) => " One bank is much worse than the other, so look at that side of the engine: its intake gasket, injectors or exhaust.",
                None => "",
            };
            if worst.abs() > 10.0 {
                result(
                    check,
                    Verdict::Fail,
                    format!("Running {side}: total trim {worst:+.1}%"),
                    format!(
                        "Beyond ±10% the engine is working hard to correct the mixture.{pattern}"
                    ),
                    readings,
                )
            } else if worst.abs() > 5.0 {
                result(
                    check,
                    Verdict::Warn,
                    format!("Slightly {side}: total trim {worst:+.1}%"),
                    format!("Within limits but higher than a healthy engine's ±5%.{pattern}"),
                    readings,
                )
            } else {
                result(
                    check,
                    Verdict::Pass,
                    format!("Mixture healthy: total trim {worst:+.1}%"),
                    "Fuel trims within ±5%: the engine is not fighting its mixture.",
                    readings,
                )
            }
        }
        Check::IntakeVacuum => {
            let Some(map) = avg(s, 0x0B) else {
                return result(
                    check,
                    Verdict::Unknown,
                    "Manifold pressure not available",
                    "This engine does not report manifold pressure.",
                    vec![],
                );
            };
            let readings = vec![
                reading("Manifold pressure", map, "kPa"),
                reading("Engine speed", rpm.unwrap_or(0.0), "rpm"),
            ];
            if map > 50.0 {
                result(check, Verdict::Fail, format!("Weak vacuum: {map:.0} kPa at idle"), "Healthy engines pull the manifold down to about 25-40 kPa at idle. A high reading backs up an intake leak or a timing problem.", readings)
            } else if map > 42.0 {
                result(
                    check,
                    Verdict::Warn,
                    format!("Vacuum on the weak side: {map:.0} kPa"),
                    "A little above the usual 25-40 kPa. Re-check when fully warm.",
                    readings,
                )
            } else {
                result(
                    check,
                    Verdict::Pass,
                    format!("Vacuum normal: {map:.0} kPa at idle"),
                    "In the usual 25-40 kPa range for a healthy engine at idle.",
                    readings,
                )
            }
        }
        Check::IdleStability => {
            let v = s.get(&0x0C).cloned().unwrap_or_default();
            let sd = spread(&v).unwrap_or(0.0);
            let readings = vec![
                reading("Average idle", rpm.unwrap_or(0.0), "rpm"),
                reading("Variation", sd, "rpm"),
            ];
            if sd > 60.0 {
                result(check, Verdict::Fail, format!("Idle is rough: ±{sd:.0} rpm"), "A steady engine holds idle within about ±30 rpm. Large swings come with misfires.", readings)
            } else if sd > 30.0 {
                result(
                    check,
                    Verdict::Warn,
                    format!("Idle a little unsteady: ±{sd:.0} rpm"),
                    "Slightly more movement than a healthy idle. Watch the misfire counters.",
                    readings,
                )
            } else {
                result(
                    check,
                    Verdict::Pass,
                    format!("Idle steady: ±{sd:.0} rpm"),
                    "Engine speed holds steady at idle.",
                    readings,
                )
            }
        }
        Check::CatalystOxygenSensors => {
            let (Some(up), Some(down)) = (s.get(&0x14), s.get(&0x15)) else {
                return result(check, Verdict::Unknown, "Oxygen sensor voltages not available", "This engine reports its oxygen sensors in another format (wide-band). Use the catalyst readiness monitor instead.", vec![]);
            };
            let (su, sd) = (spread(up).unwrap_or(0.0), spread(down).unwrap_or(0.0));
            let readings = vec![
                reading("Upstream swing", su, "V"),
                reading("Downstream swing", sd, "V"),
                reading("Downstream average", mean(down).unwrap_or(0.0), "V"),
            ];
            if su < 0.05 {
                result(
                    check,
                    Verdict::Unknown,
                    "Upstream sensor not switching yet",
                    "Let the engine warm up fully, then run this again.",
                    readings,
                )
            } else if sd > su * 0.6 {
                result(check, Verdict::Fail, "Downstream sensor copies the upstream one", "The rear sensor swings almost as much as the front one, so the converter is not storing oxygen. Rule out exhaust leaks and the rear sensor before replacing the converter.", readings)
            } else {
                result(check, Verdict::Pass, "Converter is storing oxygen", "The rear sensor holds steady while the front one switches, which is what a working converter does.", readings)
            }
        }
        Check::WarmUp => {
            let (Some(temp), Some(run)) = (avg(s, 0x05), avg(s, 0x1F)) else {
                return result(
                    check,
                    Verdict::Unknown,
                    "Temperature not available",
                    "The engine did not report coolant temperature and run time.",
                    vec![],
                );
            };
            let readings = vec![
                reading("Coolant", temp, "°C"),
                reading("Running for", run / 60.0, "min"),
            ];
            if temp >= 80.0 {
                result(
                    check,
                    Verdict::Pass,
                    format!("Warm: {temp:.0} °C"),
                    "The engine reaches normal temperature.",
                    readings,
                )
            } else if run < 600.0 {
                result(
                    check,
                    Verdict::Unknown,
                    format!("Still warming up: {temp:.0} °C"),
                    "Let it idle or drive for ten minutes, then run this again.",
                    readings,
                )
            } else {
                result(check, Verdict::Fail, format!("Too cold after {:.0} min: {temp:.0} °C", run / 60.0), "A healthy engine is above about 80 °C by now. A thermostat stuck open is the usual cause.", readings)
            }
        }
        Check::Charging => {
            let Some(v) = avg(s, 0x42) else {
                return result(
                    check,
                    Verdict::Unknown,
                    "Voltage not available",
                    "The engine module did not report its supply voltage.",
                    vec![],
                );
            };
            let readings = vec![reading("System voltage", v, "V")];
            match (running, v) {
                (true, v) if v < 13.2 => result(check, Verdict::Fail, format!("Not charging: {v:.1} V"), "With the engine running the alternator should hold 13.5-14.8 V. Check the alternator, its belt and the battery cables.", readings),
                (true, v) if v > 15.0 => result(check, Verdict::Fail, format!("Over-charging: {v:.1} V"), "Above 15 V damages the battery and electronics. The alternator regulator is the usual suspect.", readings),
                (true, v) => result(check, Verdict::Pass, format!("Charging normally: {v:.1} V"), "The alternator is holding the right voltage.", readings),
                (false, v) if v < 12.2 => result(check, Verdict::Fail, format!("Battery low: {v:.1} V"), "Below 12.2 V at rest the battery is flat or failing, and low voltage makes modules log faults. Charge and test the battery.", readings),
                (false, v) if v < 12.4 => result(check, Verdict::Warn, format!("Battery a little low: {v:.1} V"), "A healthy battery rests at 12.4-12.7 V. Charge it before diagnosing further.", readings),
                (false, v) => result(check, Verdict::Pass, format!("Battery healthy: {v:.1} V"), "Resting voltage is normal.", readings),
            }
        }
    }
}

// ---- repair records ------------------------------------------------------

/// What the technician recorded for one step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepRecord {
    pub step_id: String,
    pub title: String,
    /// "pass", "warn", "fail", "unknown", "done" or "skipped".
    pub outcome: String,
    pub note: String,
    pub result: Option<CheckResult>,
}

/// One repair, kept per VIN: the fault, what was tested, what was done and
/// whether the fix was proven.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairRecord {
    pub id: String,
    pub vin: String,
    pub code: String,
    pub module: Option<String>,
    pub title: String,
    pub started_ms: u64,
    pub finished_ms: u64,
    pub steps: Vec<StepRecord>,
    /// What was found and done, in the technician's words.
    pub fix: String,
    pub parts: String,
    /// Checks re-run after the repair.
    pub verification: Vec<CheckResult>,
    pub verified: bool,
    pub simulated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(v: &[(u8, &[f64])]) -> Samples {
        v.iter().map(|(p, x)| (*p, x.to_vec())).collect()
    }

    #[test]
    fn finds_playbooks_and_falls_back() {
        assert_eq!(playbook_for("P0171").unwrap().id, "lean");
        assert_eq!(playbook_for("P0174-00").unwrap().id, "lean");
        assert_eq!(playbook_for("P0304").unwrap().id, "misfire");
        assert_eq!(plan_for("P0420").id, "catalyst");
        let g = plan_for("C1525-64");
        assert_eq!(g.id, "general");
        assert_eq!(g.title, "Chassis fault");
        assert_eq!(plan_for("U0155").title, "Network fault");
        // Every playbook has steps, a verification and explained causes.
        for p in PLAYBOOKS {
            assert!(!p.steps.is_empty() && !p.verify.is_empty(), "{}", p.id);
            assert!(p.codes.iter().all(|c| c.len() == 5));
            // Most likely first, and every fix has a sane cost range.
            let rank = |l: Likelihood| l as u8;
            assert!(
                p.causes
                    .windows(2)
                    .all(|w| rank(w[0].likelihood) <= rank(w[1].likelihood)),
                "{}",
                p.id
            );
            assert!(
                p.causes
                    .iter()
                    .all(|c| c.fix.cost_usd.0 <= c.fix.cost_usd.1),
                "{}",
                p.id
            );
            let mut ids: Vec<_> = p.steps.iter().map(|s| s.id).collect();
            ids.dedup();
            assert_eq!(ids.len(), p.steps.len(), "{}", p.id);
        }
    }

    #[test]
    fn lean_on_both_banks_points_to_a_shared_cause() {
        let s = samples(&[
            (0x0C, &[700.0, 710.0]),
            (0x06, &[5.0, 6.0]),
            (0x07, &[9.4]),
            (0x08, &[4.0]),
            (0x09, &[8.6]),
        ]);
        let r = evaluate(Check::FuelTrims, &s);
        assert_eq!(r.verdict, Verdict::Fail);
        assert!(r.headline.contains("lean"), "{}", r.headline);
        assert!(r.detail.contains("share"));
        let s = samples(&[(0x0C, &[700.0]), (0x06, &[1.0]), (0x07, &[1.5])]);
        assert_eq!(evaluate(Check::FuelTrims, &s).verdict, Verdict::Pass);
    }

    #[test]
    fn engine_off_checks_are_unknown_except_charging() {
        let s = samples(&[
            (0x0C, &[0.0]),
            (0x06, &[0.0]),
            (0x07, &[0.0]),
            (0x42, &[11.9]),
        ]);
        assert_eq!(evaluate(Check::FuelTrims, &s).verdict, Verdict::Unknown);
        let c = evaluate(Check::Charging, &s);
        assert_eq!(c.verdict, Verdict::Fail);
        assert!(c.headline.contains("Battery low"));
    }

    #[test]
    fn catalyst_judged_by_downstream_swing() {
        let up = [0.1, 0.9, 0.1, 0.9, 0.1, 0.9];
        let good = samples(&[
            (0x0C, &[700.0]),
            (0x14, &up),
            (0x15, &[0.7, 0.69, 0.71, 0.7]),
        ]);
        assert_eq!(
            evaluate(Check::CatalystOxygenSensors, &good).verdict,
            Verdict::Pass
        );
        let bad = samples(&[(0x0C, &[700.0]), (0x14, &up), (0x15, &[0.2, 0.8, 0.2, 0.8])]);
        assert_eq!(
            evaluate(Check::CatalystOxygenSensors, &bad).verdict,
            Verdict::Fail
        );
    }

    #[test]
    fn warm_up_waits_before_failing() {
        let early = samples(&[(0x0C, &[700.0]), (0x05, &[55.0]), (0x1F, &[120.0])]);
        assert_eq!(evaluate(Check::WarmUp, &early).verdict, Verdict::Unknown);
        let late = samples(&[(0x0C, &[700.0]), (0x05, &[62.0]), (0x1F, &[900.0])]);
        assert_eq!(evaluate(Check::WarmUp, &late).verdict, Verdict::Fail);
    }
}
