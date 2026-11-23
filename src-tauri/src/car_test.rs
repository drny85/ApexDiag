//! The first real car test: a fixed, read-only sequence that shows what the
//! adapter and the car can do, with everything captured for checking later.
//!
//! Every step only reads. Nothing here clears codes, writes, codes or runs
//! a routine, so it is safe on any car with the ignition on.

use crate::{fault_text, scan_one, Client, Inner};
use protocol_core::elm327::Response;
use protocol_core::obd2::{self, Service};
use protocol_core::vehicle::{Layout, Pack};
use protocol_core::vin;
use serde::Serialize;
use serde_json::{json, Value};
use transport::elm::ClientError;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct StepInfo {
    pub id: &'static str,
    pub title: &'static str,
    /// What the step does, in plain words.
    pub what: &'static str,
    /// What a pass proves.
    pub why: &'static str,
}

pub const STEPS: &[StepInfo] = &[
    StepInfo {
        id: "adapter",
        title: "Adapter and battery",
        what: "Asks the adapter for its version and protocol, and measures the battery at the OBD port.",
        why: "Proves the Bluetooth link works both ways and the adapter is powered by the car.",
    },
    StepInfo {
        id: "vin",
        title: "Vehicle identity",
        what: "Reads the VIN from the engine computer and decodes it.",
        why: "Proves the adapter reaches the car's diagnostic network, and picks the right vehicle pack.",
    },
    StepInfo {
        id: "engine_codes",
        title: "Engine fault codes",
        what: "Reads stored, pending and permanent emissions codes and the check-engine light.",
        why: "The same codes any OBD-II tool reads. A clean read here means generic diagnostics work.",
    },
    StepInfo {
        id: "readiness",
        title: "Emissions readiness",
        what: "Reads which emissions self-tests have finished since the codes were last cleared.",
        why: "Shows whether the car would pass a plug-in inspection today.",
    },
    StepInfo {
        id: "live",
        title: "Live data",
        what: "Reads engine speed, coolant temperature and system voltage three times.",
        why: "Proves live gauges will update. Values should change a little between reads with the engine running.",
    },
    StepInfo {
        id: "freeze",
        title: "Freeze frame",
        what: "Reads the snapshot the engine stored when its first emissions code set, if there is one.",
        why: "Shows the conditions at the moment the fault happened.",
    },
    StepInfo {
        id: "engine_module",
        title: "Engine module, direct",
        what: "Talks to the engine module directly, the way the dealer tool does: identification and its own fault memory.",
        why: "This is the step generic tools can't do. On a BMW it proves extended addressing (AT CEA) works through this adapter.",
    },
    StepInfo {
        id: "modules",
        title: "All modules",
        what: "Asks each module the app knows for this car whether it is there, and reads its fault memory.",
        why: "Maps which modules answer through this adapter, so the module list can be confirmed for this car.",
    },
];

#[derive(Debug, Serialize)]
pub struct StepResult {
    pub id: &'static str,
    /// The read worked.
    pub ok: bool,
    /// Not run, for example a step that does not apply to this car.
    pub skipped: bool,
    pub headline: String,
    pub details: Vec<String>,
    pub data: Value,
    pub elapsed_ms: u64,
}

impl StepResult {
    fn new(id: &'static str) -> Self {
        Self {
            id,
            ok: true,
            skipped: false,
            headline: String::new(),
            details: vec![],
            data: Value::Null,
            elapsed_ms: 0,
        }
    }
    fn fail(mut self, headline: impl Into<String>, e: &ClientError) -> Self {
        self.ok = false;
        self.headline = headline.into();
        self.details.push(crate::explain_client_error(e));
        self
    }
}

fn text(r: &Response) -> String {
    match r {
        Response::Text(t) => t.clone(),
        Response::Ok => "OK".into(),
        Response::Error(protocol_core::elm327::AdapterError::UnknownCommand) => {
            "not supported by this adapter".into()
        }
        Response::Error(e) => format!("error: {}", e.explain()),
        Response::Frames(f) => format!("{} frames", f.len()),
    }
}

pub async fn run(inner: &mut Inner, id: &str) -> crate::CmdResult<StepResult> {
    let step = STEPS
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| crate::CmdError::new("unknown", "Unknown test step"))?;
    let simulated = inner.simulated();
    let pack = inner.pack();
    let scan_list = crate::scan_list(inner);
    let real_documented = !simulated && pack.layout == Layout::Documented;
    let c = inner.client()?;
    c.link_mut()
        .capture_note(&format!("car test step: {}", step.title));
    let started = std::time::Instant::now();
    let mut r = match step.id {
        "adapter" => adapter(c, pack).await,
        "vin" => vin_step(c).await,
        "engine_codes" => engine_codes(c).await,
        "readiness" => readiness(c).await,
        "live" => live(c).await,
        "freeze" => freeze(c).await,
        "engine_module" => engine_module(c, simulated, pack).await,
        _ => modules(c, simulated, pack, &scan_list, real_documented).await,
    };
    r.elapsed_ms = started.elapsed().as_millis() as u64;
    c.link_mut().capture_note(&format!(
        "car test step done: {} ({})",
        step.title,
        if r.skipped {
            "skipped"
        } else if r.ok {
            "ok"
        } else {
            "failed"
        }
    ));
    Ok(r)
}

async fn adapter(c: &mut Client, pack: &'static Pack) -> StepResult {
    let mut r = StepResult::new("adapter");
    let mut answers = serde_json::Map::new();
    for (cmd, label) in [
        ("AT I", "Version"),
        ("AT @1", "Description"),
        ("AT DP", "Protocol"),
        ("AT RV", "Battery at the OBD port"),
    ] {
        match c.exchange(cmd).await {
            Ok(resp) => {
                let t = text(&resp);
                r.details.push(format!("{label}: {t}"));
                answers.insert(cmd.into(), json!(t));
            }
            Err(e) => return r.fail(format!("The adapter did not answer `{cmd}`"), &e),
        }
    }
    let cea = c
        .info()
        .map(|i| i.supports_extended_addressing)
        .unwrap_or(false);
    r.details.push(format!(
        "Extended addressing (AT CEA): {}",
        if cea { "supported" } else { "not supported" }
    ));
    let volts = c.battery_voltage().await.ok();
    r.headline = match volts {
        Some(v) if v < 11.8 => {
            format!("Adapter answers. Battery low at {v:.1} V: charge it before longer jobs.")
        }
        Some(v) => format!("Adapter answers. Battery {v:.1} V."),
        None => "Adapter answers.".into(),
    };
    if !cea && pack.id == "bmw_e70" {
        r.details.push("This adapter can't reach BMW modules directly. Engine OBD-II still works; a K+DCAN cable or an ELM327 v1.4+ with AT CEA can reach the rest.".into());
    }
    r.data = json!({ "answers": answers, "cea": cea, "battery_v": volts });
    r
}

async fn vin_step(c: &mut Client) -> StepResult {
    let mut r = StepResult::new("vin");
    match c.read_vin().await {
        Ok(v) => {
            let info = vin::decode(&v, crate::current_year());
            r.headline = format!("VIN {v}");
            if let Some(m) = info.manufacturer {
                r.details.push(format!("Built by {m}"));
            }
            if let Some(y) = info.model_year {
                r.details.push(format!("Model year {y}"));
            }
            if info.check_digit_ok == Some(false) && info.check_digit_required {
                r.details
                    .push("The check digit doesn't match: the VIN may have been misread.".into());
            }
            r.data = json!({ "vin": v, "decoded": info });
        }
        Err(e) => return r.fail("The VIN could not be read", &e),
    }
    r
}

async fn engine_codes(c: &mut Client) -> StepResult {
    let mut r = StepResult::new("engine_codes");
    let mut all = serde_json::Map::new();
    let mut total = 0;
    for (svc, label) in [
        (Service::StoredDtcs, "Stored"),
        (Service::PendingDtcs, "Pending"),
        (Service::PermanentDtcs, "Permanent"),
    ] {
        match c.read_obd_dtcs(svc).await {
            Ok(codes) => {
                total += codes.len();
                let list: Vec<String> = codes
                    .iter()
                    .map(|d| match &d.description {
                        Some(t) => format!("{} {t}", d.code),
                        None => d.code.clone(),
                    })
                    .collect();
                r.details.push(format!(
                    "{label}: {}",
                    if list.is_empty() {
                        "none".into()
                    } else {
                        list.join(", ")
                    }
                ));
                all.insert(label.to_lowercase(), json!(list));
            }
            // Permanent codes (service 0A) are missing on older cars.
            Err(_) if svc == Service::PermanentDtcs => {
                r.details
                    .push("Permanent: not supported by this car (normal before about 2010)".into());
            }
            Err(e) => return r.fail(format!("{label} codes could not be read"), &e),
        }
    }
    let mil = match c.obd(&[0x01, 0x01]).await {
        Ok(replies) => replies
            .first()
            .and_then(|(_, p)| obd2::decode_mil_status(p).ok()),
        Err(_) => None,
    };
    r.headline = match (total, mil.map(|m| m.mil_on)) {
        (0, Some(false)) => "No engine codes, check-engine light off.".into(),
        (n, Some(true)) => format!("{n} code(s) read, check-engine light on."),
        (n, _) => format!("{n} code(s) read."),
    };
    r.data = json!({ "codes": all, "mil": mil });
    r
}

async fn readiness(c: &mut Client) -> StepResult {
    let mut r = StepResult::new("readiness");
    match c.read_readiness().await {
        Ok(rd) => {
            let incomplete = rd.incomplete();
            r.headline = if incomplete == 0 {
                "Every supported monitor has finished.".into()
            } else {
                format!("{incomplete} monitor(s) not finished yet.")
            };
            for m in rd.monitors.iter().filter(|m| m.supported) {
                r.details.push(format!(
                    "{}: {}",
                    m.name,
                    if m.complete { "ready" } else { "not ready" }
                ));
            }
            r.data = json!(rd);
        }
        Err(e) => return r.fail("Readiness could not be read", &e),
    }
    r
}

async fn live(c: &mut Client) -> StepResult {
    let mut r = StepResult::new("live");
    let mut samples = vec![];
    for _ in 0..3 {
        let mut row = serde_json::Map::new();
        for (pid, name) in [(0x0C, "rpm"), (0x05, "coolant_c"), (0x42, "volts")] {
            match c.read_pid(pid).await {
                Ok(v) => {
                    row.insert(name.into(), json!(v));
                }
                Err(ClientError::NoReply(_)) => {
                    row.insert(name.into(), Value::Null);
                }
                Err(e) => return r.fail("Live data stopped", &e),
            }
        }
        samples.push(Value::Object(row));
    }
    let supported = c.supported_pids().await.map(|p| p.len()).unwrap_or(0);
    let last = samples.last().cloned().unwrap_or_default();
    let num = |k: &str| last.get(k).and_then(Value::as_f64);
    r.headline = match (num("rpm"), num("coolant_c")) {
        (Some(rpm), Some(t)) => format!("{rpm:.0} rpm, coolant {t:.0} °C."),
        _ => "Some values did not answer.".into(),
    };
    if let Some(v) = num("volts") {
        r.details.push(format!("System voltage {v:.2} V"));
    }
    r.details
        .push(format!("{supported} live values supported by this car"));
    if num("rpm") == Some(0.0) {
        r.details
            .push("Engine is off. Start it for this step to show moving values.".into());
    }
    r.data = json!({ "samples": samples, "supported": supported });
    r
}

async fn freeze(c: &mut Client) -> StepResult {
    let mut r = StepResult::new("freeze");
    match c.read_freeze_frame().await {
        Ok(Some(f)) => {
            r.headline = format!("Freeze frame stored by {}.", f.dtc);
            for (pid, v) in &f.values {
                let name = obd2::pid_info(*pid).map(|p| p.name).unwrap_or("PID");
                r.details.push(format!("{name}: {v:.1}"));
            }
            r.data = json!(f);
        }
        Ok(None) => r.headline = "No freeze frame stored.".into(),
        Err(e) => return r.fail("The freeze frame could not be read", &e),
    }
    r
}

async fn engine_module(c: &mut Client, simulated: bool, pack: &'static Pack) -> StepResult {
    let mut r = StepResult::new("engine_module");
    let Some(m) = crate::engine_module(pack) else {
        r.skipped = true;
        r.headline = "No vehicle pack for this car, so there is no direct module to ask.".into();
        return r;
    };
    if !simulated && pack.layout == Layout::Demo && !m.is_obd_legislated() {
        r.skipped = true;
        r.headline = format!(
            "{}'s address is not confirmed for a real car yet, so it is not asked.",
            m.name
        );
        return r;
    }
    r.details.push(format!(
        "{} ({}) over {}",
        m.name,
        m.description,
        crate::link_label(m)
    ));
    match c.probe(m).await {
        Ok(true) => {}
        Ok(false) => {
            r.ok = false;
            r.headline = format!("{} did not answer.", m.name);
            r.details.push("Check the ignition is on. On a BMW this usually means the adapter does not pass extended addressing (AT CEA) through.".into());
            return r;
        }
        Err(e) => return r.fail(format!("{} could not be reached", m.name), &e),
    }
    let ident = c.read_module_identification(m).await;
    match &ident {
        Ok(t) => r.details.push(format!("Identification: {t}")),
        Err(e) => r.details.push(format!(
            "Identification not read: {}",
            crate::explain_client_error(e)
        )),
    }
    match c.read_module_dtcs(m).await {
        Ok(dtcs) => {
            r.headline = format!(
                "{} answers directly, {} fault(s) in its memory.",
                m.name,
                dtcs.len()
            );
            for d in &dtcs {
                let t = fault_text(simulated, pack, m.address, &d.code)
                    .unwrap_or("meaning to be confirmed from the capture");
                r.details
                    .push(format!("{} (status {:02X}): {t}", d.code, d.status));
            }
            r.data = json!({ "module": m.name, "ident": ident.ok(), "faults": dtcs });
        }
        Err(e) => {
            return r.fail(
                format!("{} answered but its fault memory could not be read", m.name),
                &e,
            )
        }
    }
    r
}

async fn modules(
    c: &mut Client,
    simulated: bool,
    pack: &'static Pack,
    list: &[&'static protocol_core::vehicle::Module],
    real_documented: bool,
) -> StepResult {
    let mut r = StepResult::new("modules");
    let mut rows = vec![];
    let mut present = 0;
    for m in list {
        let s = scan_one(c, simulated, pack, m).await;
        if s.present {
            present += 1;
        }
        r.details.push(format!(
            "{}: {}",
            m.name,
            match (s.present, s.faults) {
                (false, _) => "no reply".to_string(),
                (true, Some(n)) => format!("answers, {n} fault(s)"),
                (true, None) => "answers, fault memory not read".to_string(),
            }
        ));
        rows.push(json!({ "module": m.name, "address": m.address, "present": s.present, "faults": s.codes }));
    }
    r.headline = format!("{present} of {} modules answered.", list.len());
    if !simulated && !real_documented {
        r.details.push("Only the legislated engine and transmission addresses are asked on this car until its module map is confirmed.".into());
    }
    r.ok = present > 0;
    r.data = json!(rows);
    r
}
