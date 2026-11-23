//! Full scan: every reading the app can take in one pass, each turned into
//! a plain-language card.
//!
//! A reading that looks fine says what that part of the car does, so the
//! owner learns the car as they go. A reading that looks wrong says what is
//! wrong, what usually fixes it, and which Repair Copilot guide to open.
//!
//! The rules are generic OBD-II facts and the app's own module scan, kept
//! here so they are the same on every platform and can be tested. Nothing
//! here invents a manufacturer fault meaning: module fault text comes from
//! the caller, and a code without text says so. Values that need the engine
//! running (fuel trims, charging) are only judged when it is running; a
//! single snapshot that cannot be judged (oxygen sensors, airflow) is shown
//! as information, never as a pass.

use crate::copilot::{self, Difficulty};
use crate::inspection::Lang;
use crate::obd2::Readiness;
use crate::vehicle::System;
use crate::vin::VinInfo;
use serde::{Deserialize, Serialize};

/// How a reading came out. Ordered from least to most urgent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Could not be read, or needs a condition that wasn't met.
    NotRead,
    /// A value worth knowing that one snapshot cannot judge.
    Info,
    Good,
    Attention,
    Problem,
}

/// Where a reading belongs on the results page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    Engine,
    Emissions,
    Electrical,
    Modules,
    Vehicle,
}

/// How a module reports one of its stored faults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeState {
    /// Failing right now.
    Active,
    /// Seen once, not confirmed yet.
    Pending,
    /// In memory from earlier.
    Stored,
}

impl CodeState {
    /// From a UDS DTC status byte: bit 0 testFailed, bit 2 pending, bit 3
    /// confirmed. KWP2000 status bytes vary by maker, so they are `Stored`.
    pub fn from_uds(status: u8) -> Self {
        if status & 0x01 != 0 {
            Self::Active
        } else if status & 0x04 != 0 && status & 0x08 == 0 {
            Self::Pending
        } else {
            Self::Stored
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleCode {
    pub code: String,
    pub description: Option<String>,
    pub state: CodeState,
}

/// One module as the scan found it.
#[derive(Debug, Clone, Serialize)]
pub struct ModuleRead {
    pub name: String,
    pub description: String,
    /// What it controls, from the vehicle pack.
    pub role: String,
    pub system: System,
    pub present: bool,
    pub codes: Vec<ModuleCode>,
}

/// An engine (OBD-II) code with where it was found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCode {
    pub code: String,
    pub description: Option<String>,
}

/// Everything the scan read.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Input {
    pub vin: Option<VinInfo>,
    /// "BMW X5", for sentences.
    pub vehicle: String,
    pub is_bmw: bool,
    pub simulated: bool,
    pub adapter_version: Option<String>,
    /// Whether the adapter accepted `AT CEA` (BMW module addressing).
    pub adapter_extended: Option<bool>,
    pub readiness: Option<Readiness>,
    /// None when the engine codes could not be read.
    pub stored: Option<Vec<EngineCode>>,
    pub pending: Vec<EngineCode>,
    pub permanent: Vec<EngineCode>,
    /// The code that set freeze frame 0, and the values captured.
    pub freeze_frame: Option<(String, Vec<(u8, f64)>)>,
    pub battery_v: Option<f64>,
    pub engine_running: Option<bool>,
    /// Live values read once during the scan: (PID, value).
    pub live: Vec<(u8, f64)>,
    pub modules: Vec<ModuleRead>,
    /// Modules not asked because their address isn't confirmed yet.
    pub modules_not_scanned: usize,
}

impl Input {
    fn pid(&self, pid: u8) -> Option<f64> {
        self.live.iter().find(|(p, _)| *p == pid).map(|(_, v)| *v)
    }
}

/// A label and a value, for the detail rows under a card.
#[derive(Debug, Clone, Serialize)]
pub struct Fact {
    pub label: String,
    pub value: String,
}

/// A Repair Copilot guide to open from a card.
#[derive(Debug, Clone, Serialize)]
pub struct Guide {
    /// The code the Copilot plans for.
    pub code: String,
    /// Whether a dedicated playbook exists (else a general plan).
    pub playbook: bool,
    pub title: String,
}

/// One reading on the results page.
#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: String,
    pub area: Area,
    pub title: String,
    pub value: Option<String>,
    pub status: Status,
    /// One line: the verdict in words.
    pub headline: String,
    /// What this part of the car does, in plain words.
    pub what: String,
    /// What is wrong, for Attention and Problem.
    pub problem: Option<String>,
    /// What to do about it, in order.
    pub fix: Vec<String>,
    pub facts: Vec<Fact>,
    pub guide: Option<Guide>,
    /// A module this card is about, for the Modules page.
    pub module: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Counts {
    pub problem: usize,
    pub attention: usize,
    pub good: usize,
    pub info: usize,
    pub not_read: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The most urgent status among the readings (never NotRead unless
    /// nothing at all could be read).
    pub overall: Status,
    pub title: String,
    pub summary: String,
    pub counts: Counts,
    pub items: Vec<Item>,
    /// Things to do for a more complete scan next time.
    pub tips: Vec<String>,
}

// ---- text helpers ----------------------------------------------------------

macro_rules! tr {
    ($lang:expr, $en:literal, $es:literal $(, $arg:expr)* $(,)?) => {
        match $lang {
            Lang::En => format!($en $(, $arg)*),
            Lang::Es => format!($es $(, $arg)*),
        }
    };
}

fn pick(lang: Lang, en: &str, es: &str) -> String {
    match lang {
        Lang::En => en.to_string(),
        Lang::Es => es.to_string(),
    }
}

fn item(id: impl Into<String>, area: Area, title: String, status: Status) -> Item {
    Item {
        id: id.into(),
        area,
        title,
        value: None,
        status,
        headline: String::new(),
        what: String::new(),
        problem: None,
        fix: Vec::new(),
        facts: Vec::new(),
        guide: None,
        module: None,
    }
}

fn fact(label: String, value: impl Into<String>) -> Fact {
    Fact {
        label,
        value: value.into(),
    }
}

fn whole(n: f64) -> String {
    let n = n.round() as i64;
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

fn difficulty(lang: Lang, d: Difficulty) -> String {
    match d {
        Difficulty::Diy => pick(lang, "DIY", "hazlo tú"),
        Difficulty::Experienced => pick(lang, "experienced DIY", "con experiencia"),
        Difficulty::Shop => pick(lang, "shop job", "trabajo de taller"),
    }
}

/// The Copilot guide for a code, and its most common fixes as steps.
fn guide_for(code: &str, lang: Lang) -> (Guide, Vec<String>) {
    match copilot::playbook_for(code) {
        Some(p) => {
            let fixes = p
                .causes
                .iter()
                .take(2)
                .enumerate()
                .map(|(n, c)| {
                    let (lo, hi) = c.fix.cost_usd;
                    let lead = match (n, lang) {
                        (0, Lang::En) => "Most likely",
                        (_, Lang::En) => "Next most likely",
                        (0, Lang::Es) => "Lo más probable",
                        (_, Lang::Es) => "Lo siguiente más probable",
                    };
                    tr!(
                        lang,
                        "{lead}: {}. {} ({}, about ${lo}-{hi}).",
                        "{lead}: {}. {} ({}, unos ${lo}-{hi}).",
                        c.name,
                        c.fix.action,
                        difficulty(lang, c.fix.difficulty),
                    )
                })
                .collect();
            (
                Guide {
                    code: code.to_string(),
                    playbook: true,
                    title: p.title.to_string(),
                },
                fixes,
            )
        }
        None => (
            Guide {
                code: code.to_string(),
                playbook: false,
                title: copilot::general_plan(code).title.to_string(),
            },
            vec![tr!(
                lang,
                "Open the Repair Copilot for a step-by-step plan to find the cause before replacing parts.",
                "Abre el Copiloto de reparación para seguir un plan paso a paso y encontrar la causa antes de cambiar piezas."
            )],
        ),
    }
}

// ---- vehicle -----------------------------------------------------------------

fn vehicle(i: &Input, lang: Lang, out: &mut Vec<Item>) {
    let title = pick(
        lang,
        "Vehicle identity (VIN)",
        "Identidad del vehículo (VIN)",
    );
    let what = pick(
        lang,
        "The VIN is the car's fingerprint. The engine module stores its own copy, so reading it over the plug proves which car this computer belongs to, and tells the app which brand's modules to ask for.",
        "El VIN es la huella digital del auto. El módulo del motor guarda su propia copia, así que leerlo por el conector demuestra a qué auto pertenece esta computadora y le dice a la app qué módulos de marca consultar.",
    );
    let Some(v) = &i.vin else {
        let mut it = item("vin", Area::Vehicle, title, Status::NotRead);
        it.headline = pick(
            lang,
            "The car did not report its VIN.",
            "El auto no informó su VIN.",
        );
        it.what = what;
        it.fix = vec![pick(
            lang,
            "Many cars before 2005 don't report the VIN over OBD-II. Everything else still works; the app uses generic OBD-II.",
            "Muchos autos anteriores a 2005 no informan el VIN por OBD-II. Todo lo demás funciona; la app usa OBD-II genérico.",
        )];
        out.push(it);
        return;
    };
    let mut facts = vec![];
    if let Some(m) = v.manufacturer {
        facts.push(fact(pick(lang, "Maker", "Fabricante"), m));
    }
    if let Some(y) = v.model_year {
        facts.push(fact(pick(lang, "Model year", "Año modelo"), y.to_string()));
    }
    if let Some(c) = v.country {
        facts.push(fact(pick(lang, "Built in", "Fabricado en"), c));
    }
    if let Some(p) = v.plant {
        facts.push(fact(
            pick(lang, "Plant code", "Código de planta"),
            p.to_string(),
        ));
    }
    facts.push(fact(pick(lang, "Serial", "Serie"), v.serial.clone()));
    let bad_check = v.check_digit_required && v.check_digit_ok == Some(false);
    let status = if v.demo {
        Status::Info
    } else if !v.well_formed || bad_check {
        Status::Attention
    } else {
        Status::Good
    };
    let mut it = item("vin", Area::Vehicle, title, status);
    it.value = Some(v.vin.clone());
    it.what = what;
    it.facts = facts;
    it.headline = if v.demo {
        pick(
            lang,
            "Practice car: an invented VIN.",
            "Auto de práctica: un VIN inventado.",
        )
    } else if !v.well_formed {
        pick(
            lang,
            "The VIN the engine module reports is not a valid 17-character VIN.",
            "El VIN que informa el módulo del motor no es un VIN válido de 17 caracteres.",
        )
    } else if bad_check {
        pick(
            lang,
            "The VIN's check digit doesn't add up.",
            "El dígito verificador del VIN no cuadra.",
        )
    } else {
        pick(
            lang,
            "Read from the engine module and decoded.",
            "Leído del módulo del motor y decodificado.",
        )
    };
    if status == Status::Attention {
        it.problem = Some(pick(
            lang,
            "On North American cars the 9th character is a checksum of the others. A mismatch can mean a misread, or an engine module that came from another car.",
            "En autos de Norteamérica el 9.º carácter es una suma de verificación de los demás. Si no coincide puede ser un error de lectura o un módulo del motor que vino de otro auto.",
        ));
        it.fix = vec![pick(
            lang,
            "Compare it with the VIN plate at the base of the windscreen and on the title. If they differ, ask about the engine module's history.",
            "Compáralo con la placa del VIN en la base del parabrisas y con el título. Si no coinciden, pregunta por el historial del módulo del motor.",
        )];
    }
    out.push(it);

    if let Some(km) = i.pid(0xA6) {
        let mut it = item(
            "odometer",
            Area::Vehicle,
            pick(lang, "Odometer", "Odómetro"),
            Status::Info,
        );
        it.value = Some(format!("{} km · {} mi", whole(km), whole(km * 0.621_371)));
        it.headline = pick(
            lang,
            "As the engine module counts it.",
            "Según lo cuenta el módulo del motor.",
        );
        it.what = pick(
            lang,
            "Newer cars report total distance over OBD-II. Compare it with the dashboard: a big difference is a sign the cluster was changed or tampered with.",
            "Los autos más nuevos informan la distancia total por OBD-II. Compárala con el tablero: una gran diferencia indica que el tablero se cambió o se manipuló.",
        );
        out.push(it);
    }
}

fn adapter(i: &Input, lang: Lang, out: &mut Vec<Item>) {
    let Some(version) = &i.adapter_version else {
        return;
    };
    let blocked = i.is_bmw && i.adapter_extended == Some(false);
    let mut it = item(
        "adapter",
        Area::Vehicle,
        pick(lang, "Diagnostic adapter", "Adaptador de diagnóstico"),
        if blocked {
            Status::Problem
        } else {
            Status::Info
        },
    );
    it.value = Some(version.clone());
    it.what = pick(
        lang,
        "The plug-in adapter translates between this app and the car's network. Cheap ELM327-style adapters handle generic OBD-II well; reaching every brand module needs one that supports the brand's addressing.",
        "El adaptador conectado traduce entre esta app y la red del auto. Los adaptadores tipo ELM327 baratos manejan bien el OBD-II genérico; para llegar a todos los módulos de la marca hace falta uno que soporte el direccionamiento de esa marca.",
    );
    if blocked {
        it.headline = pick(
            lang,
            "This adapter can't reach BMW's own modules.",
            "Este adaptador no puede llegar a los módulos propios de BMW.",
        );
        it.problem = Some(pick(
            lang,
            "It refused the extended-addressing command (AT CEA) that BMW modules need, so only the engine's generic OBD-II data could be read. The other modules were not skipped because of a fault in the car.",
            "Rechazó el comando de direccionamiento extendido (AT CEA) que necesitan los módulos BMW, así que solo se pudieron leer los datos OBD-II genéricos del motor. Los demás módulos no se omitieron por una falla del auto.",
        ));
        it.fix = vec![pick(
            lang,
            "Use an adapter built on an STN chip (OBDLink MX+ or CX) or a K+DCAN cable to scan every BMW module.",
            "Usa un adaptador con chip STN (OBDLink MX+ o CX) o un cable K+DCAN para escanear todos los módulos BMW.",
        )];
    } else {
        it.headline = pick(
            lang,
            "Connected and answering.",
            "Conectado y respondiendo.",
        );
    }
    out.push(it);
}

// ---- engine ------------------------------------------------------------------

fn engine_codes(i: &Input, lang: Lang, out: &mut Vec<Item>) {
    let mil = i.readiness.as_ref().map(|r| r.mil_on);
    // The check-engine light.
    let mut light = item(
        "mil",
        Area::Engine,
        pick(lang, "Check-engine light", "Luz de revisar motor"),
        match mil {
            None => Status::NotRead,
            Some(true) => Status::Problem,
            Some(false) => Status::Good,
        },
    );
    light.what = pick(
        lang,
        "The engine module turns this light on when a self-test finds a fault that can raise emissions or harm the engine. If it flashes while driving, a misfire is damaging the catalytic converter: slow down and stop soon.",
        "El módulo del motor enciende esta luz cuando una autoprueba encuentra una falla que puede aumentar las emisiones o dañar el motor. Si parpadea al manejar, una falla de encendido está dañando el catalizador: baja la velocidad y detente pronto.",
    );
    match mil {
        None => {
            light.headline = pick(lang, "Could not be read.", "No se pudo leer.");
        }
        Some(true) => {
            light.value = Some(pick(lang, "On", "Encendida"));
            light.headline = pick(
                lang,
                "On: the engine has a confirmed fault.",
                "Encendida: el motor tiene una falla confirmada.",
            );
            light.problem = Some(pick(
                lang,
                "A confirmed emissions fault is stored. The car would fail a plug-in emissions inspection while the light is on.",
                "Hay una falla de emisiones confirmada. El auto no pasaría una inspección de emisiones mientras la luz esté encendida.",
            ));
            light.fix = vec![pick(
                lang,
                "Work through the engine codes below. After the repair the light goes out by itself after a few drives, or clear it once you've confirmed the fix.",
                "Revisa los códigos del motor de abajo. Después de la reparación la luz se apaga sola tras algunos viajes, o bórrala cuando hayas confirmado el arreglo.",
            )];
            if let Some(km) = i.pid(0x21).filter(|k| *k > 0.0) {
                light.facts.push(fact(
                    pick(
                        lang,
                        "Driven with the light on",
                        "Recorrido con la luz encendida",
                    ),
                    format!("{} km", whole(km)),
                ));
            }
        }
        Some(false) => {
            light.value = Some(pick(lang, "Off", "Apagada"));
            light.headline = pick(
                lang,
                "Off: no confirmed engine fault.",
                "Apagada: no hay falla del motor confirmada.",
            );
        }
    }
    out.push(light);

    let Some(stored) = &i.stored else {
        let mut it = item(
            "engine_codes",
            Area::Engine,
            pick(lang, "Engine codes", "Códigos del motor"),
            Status::NotRead,
        );
        it.headline = pick(
            lang,
            "The engine module did not answer the code request.",
            "El módulo del motor no respondió a la solicitud de códigos.",
        );
        it.what = engine_codes_what(lang);
        it.fix = vec![pick(
            lang,
            "Turn the ignition on (engine off is fine), wait 10 seconds and scan again.",
            "Pon el encendido en ON (con el motor apagado está bien), espera 10 segundos y escanea de nuevo.",
        )];
        out.push(it);
        return;
    };

    // One card per code, with where it was found.
    let mut seen: Vec<&str> = Vec::new();
    for c in stored.iter().chain(&i.pending).chain(&i.permanent) {
        if seen.contains(&c.code.as_str()) {
            continue;
        }
        seen.push(&c.code);
        let is_stored = stored.iter().any(|x| x.code == c.code);
        let is_pending = i.pending.iter().any(|x| x.code == c.code);
        let is_permanent = i.permanent.iter().any(|x| x.code == c.code);
        let status = if is_stored || is_permanent {
            Status::Problem
        } else {
            Status::Attention
        };
        let mut it = item(
            format!("code_{}", c.code),
            Area::Engine,
            c.code.clone(),
            status,
        );
        let mut tags = vec![];
        if is_stored {
            tags.push(pick(lang, "stored", "guardado"));
        }
        if is_pending {
            tags.push(pick(lang, "pending", "pendiente"));
        }
        if is_permanent {
            tags.push(pick(lang, "permanent", "permanente"));
        }
        it.value = Some(tags.join(" · "));
        it.headline = c.description.clone().unwrap_or_else(|| {
            pick(
                lang,
                "No description in the app's library yet.",
                "Todavía sin descripción en la biblioteca de la app.",
            )
        });
        it.what = engine_codes_what(lang);
        it.problem = Some(if is_stored {
            pick(
                lang,
                "The engine module confirmed this fault on more than one drive.",
                "El módulo del motor confirmó esta falla en más de un viaje.",
            )
        } else if is_permanent {
            pick(
                lang,
                "A permanent code stays until the car itself proves the fault is fixed, so clearing it does nothing. It will fail an emissions inspection until then.",
                "Un código permanente se queda hasta que el propio auto comprueba que la falla se arregló, así que borrarlo no sirve. Hasta entonces no pasará una inspección de emisiones.",
            )
        } else {
            pick(
                lang,
                "Seen once and not confirmed yet. If it shows up again on the next drive it becomes a stored code and the light comes on.",
                "Se detectó una vez y aún no está confirmado. Si vuelve a aparecer en el siguiente viaje pasa a ser un código guardado y se enciende la luz.",
            )
        });
        let (guide, fixes) = guide_for(&c.code, lang);
        it.fix = fixes;
        if let Some((ff, values)) = &i.freeze_frame {
            if ff == &c.code && !values.is_empty() {
                it.fix.push(pick(
                    lang,
                    "Its freeze frame (below) shows the conditions when it set: try to recreate them to confirm the fix.",
                    "Su cuadro congelado (abajo) muestra las condiciones cuando se activó: intenta recrearlas para confirmar el arreglo.",
                ));
            }
        }
        it.guide = Some(guide);
        out.push(it);
    }
    if seen.is_empty() {
        let mut it = item(
            "engine_codes",
            Area::Engine,
            pick(lang, "Engine codes", "Códigos del motor"),
            Status::Good,
        );
        it.value = Some("0".into());
        it.headline = pick(
            lang,
            "No stored, pending or permanent engine codes.",
            "No hay códigos del motor guardados, pendientes ni permanentes.",
        );
        it.what = engine_codes_what(lang);
        out.push(it);
    }

    // Freeze frame: the snapshot taken when the first code set.
    if let Some((code, values)) = &i.freeze_frame {
        let mut it = item(
            "freeze_frame",
            Area::Engine,
            pick(lang, "Freeze frame", "Cuadro congelado"),
            Status::Info,
        );
        it.value = Some(code.clone());
        it.headline = tr!(
            lang,
            "What the engine was doing when {} set.",
            "Lo que hacía el motor cuando se activó {}.",
            code
        );
        it.what = pick(
            lang,
            "When a code sets, the engine module saves a snapshot of the key sensors at that moment. It tells you whether the fault happened cold or warm, at idle or on the highway, which narrows down the cause.",
            "Cuando se activa un código, el módulo del motor guarda una foto de los sensores principales en ese momento. Indica si la falla ocurrió en frío o en caliente, en ralentí o en carretera, lo que ayuda a encontrar la causa.",
        );
        it.facts = values
            .iter()
            .filter_map(|(pid, v)| {
                let (name, value) = live_label(*pid, *v, lang)?;
                Some(fact(name, value))
            })
            .collect();
        out.push(it);
    }
}

fn engine_codes_what(lang: Lang) -> String {
    pick(
        lang,
        "The engine module runs hundreds of self-tests and stores a standard code (P0xxx) when one fails. Pending means seen once; stored means confirmed; permanent means it stays until the car proves the repair.",
        "El módulo del motor hace cientos de autopruebas y guarda un código estándar (P0xxx) cuando una falla. Pendiente significa que se vio una vez; guardado, que está confirmado; permanente, que se queda hasta que el auto compruebe la reparación.",
    )
}

/// Name and formatted value of a live PID, in the report's language.
fn live_label(pid: u8, v: f64, lang: Lang) -> Option<(String, String)> {
    let (en, es, value) = match pid {
        0x04 => ("Engine load", "Carga del motor", format!("{v:.0} %")),
        0x05 => ("Coolant", "Refrigerante", format!("{v:.0} °C")),
        0x06 => (
            "Short-term trim B1",
            "Ajuste corto B1",
            format!("{v:+.1} %"),
        ),
        0x07 => ("Long-term trim B1", "Ajuste largo B1", format!("{v:+.1} %")),
        0x08 => (
            "Short-term trim B2",
            "Ajuste corto B2",
            format!("{v:+.1} %"),
        ),
        0x09 => ("Long-term trim B2", "Ajuste largo B2", format!("{v:+.1} %")),
        0x0B => (
            "Intake pressure",
            "Presión de admisión",
            format!("{v:.0} kPa"),
        ),
        0x0C => ("Engine speed", "Revoluciones", format!("{v:.0} rpm")),
        0x0D => ("Speed", "Velocidad", format!("{v:.0} km/h")),
        0x0F => ("Intake air", "Aire de admisión", format!("{v:.0} °C")),
        0x11 => ("Throttle", "Acelerador", format!("{v:.0} %")),
        _ => return None,
    };
    Some((pick(lang, en, es), value))
}

/// Live readings: judged only when the conditions make them meaningful.
fn live(i: &Input, lang: Lang, out: &mut Vec<Item>) {
    let running = i.engine_running == Some(true);
    let coolant = i.pid(0x05);
    let warm = coolant.is_some_and(|c| c >= 70.0);

    if let Some(c) = coolant {
        let run_time = i.pid(0x1F).unwrap_or(0.0);
        let (status, headline, problem, fix, guide) = if running && c >= 112.0 {
            (
                Status::Problem,
                pick(lang, "Running hot.", "Está muy caliente."),
                Some(pick(
                    lang,
                    "Above about 110 °C most engines are overheating, which can warp the head and blow the head gasket.",
                    "Por encima de unos 110 °C la mayoría de los motores se están sobrecalentando, lo que puede deformar la culata y dañar el empaque.",
                )),
                vec![
                    pick(lang, "Stop the engine and let it cool. Never open a hot cooling system.", "Apaga el motor y déjalo enfriar. Nunca abras el sistema de enfriamiento caliente."),
                    pick(lang, "Check the coolant level cold, look for leaks, and confirm the radiator fan runs.", "Revisa el nivel de refrigerante en frío, busca fugas y confirma que el ventilador del radiador funcione."),
                    pick(lang, "A weak water pump or stuck thermostat are the usual causes after the level is right.", "Si el nivel está bien, las causas habituales son una bomba de agua débil o un termostato trabado."),
                ],
                None,
            )
        } else if running && run_time > 900.0 && c < 70.0 {
            let (g, mut fixes) = guide_for("P0128", lang);
            fixes.insert(0, pick(lang, "A thermostat stuck open is the usual cause: the engine never warms up fully, which costs fuel and heater output.", "La causa habitual es un termostato trabado abierto: el motor nunca se calienta del todo, lo que gasta más combustible y calienta menos la cabina."));
            fixes.truncate(2);
            (
                Status::Attention,
                pick(lang, "Still cool after 15 minutes of running.", "Sigue frío después de 15 minutos encendido."),
                Some(pick(
                    lang,
                    "After a quarter hour the coolant should be near operating temperature (about 85-105 °C).",
                    "Después de un cuarto de hora el refrigerante debería estar cerca de su temperatura de trabajo (unos 85-105 °C).",
                )),
                fixes,
                Some(g),
            )
        } else if running {
            (
                Status::Good,
                if warm {
                    pick(
                        lang,
                        "At operating temperature.",
                        "A temperatura de trabajo.",
                    )
                } else {
                    pick(lang, "Warming up normally.", "Calentándose con normalidad.")
                },
                None,
                vec![],
                None,
            )
        } else {
            (
                Status::Info,
                pick(
                    lang,
                    "Engine off: this is the resting temperature.",
                    "Motor apagado: es la temperatura en reposo.",
                ),
                None,
                vec![],
                None,
            )
        };
        let mut it = item(
            "coolant",
            Area::Engine,
            pick(lang, "Coolant temperature", "Temperatura del refrigerante"),
            status,
        );
        it.value = Some(format!("{c:.0} °C"));
        it.headline = headline;
        it.what = pick(
            lang,
            "Coolant carries heat away from the engine to the radiator. The engine module uses this temperature to set fuel, idle speed and the cooling fan. Most engines settle at 85-105 °C; some BMWs run hotter on purpose to save fuel.",
            "El refrigerante lleva el calor del motor al radiador. El módulo del motor usa esta temperatura para ajustar el combustible, el ralentí y el ventilador. La mayoría de los motores se estabilizan en 85-105 °C; algunos BMW trabajan más calientes a propósito para ahorrar combustible.",
        );
        it.problem = problem;
        it.fix = fix;
        it.guide = guide;
        out.push(it);
    }

    // Fuel trims, one card per bank.
    for (bank, st_pid, lt_pid, lean, rich) in [
        (1u8, 0x06u8, 0x07u8, "P0171", "P0172"),
        (2, 0x08, 0x09, "P0174", "P0175"),
    ] {
        let (Some(st), Some(lt)) = (i.pid(st_pid), i.pid(lt_pid)) else {
            continue;
        };
        let total = st + lt;
        let title = tr!(
            lang,
            "Fuel trim, bank {bank}",
            "Ajuste de combustible, banco {bank}"
        );
        let what = pick(
            lang,
            "Fuel trim is how much the engine module is correcting the fuel to keep the mixture right. Near zero means the air and fuel it measures agree. Positive means it is adding fuel because it sees extra air (lean); negative means it is taking fuel away (rich).",
            "El ajuste de combustible es cuánto corrige el módulo del motor el combustible para mantener la mezcla correcta. Cerca de cero significa que el aire y el combustible que mide coinciden. Positivo significa que agrega combustible porque ve aire de más (pobre); negativo, que quita combustible (rica).",
        );
        let facts = vec![
            fact(
                pick(lang, "Short-term", "Corto plazo"),
                format!("{st:+.1} %"),
            ),
            fact(
                pick(lang, "Long-term (learned)", "Largo plazo (aprendido)"),
                format!("{lt:+.1} %"),
            ),
        ];
        let (status, headline) = if !running {
            (
                Status::NotRead,
                pick(
                    lang,
                    "Needs the engine running to judge.",
                    "Se necesita el motor encendido para evaluarlo.",
                ),
            )
        } else if !warm {
            (
                Status::NotRead,
                pick(
                    lang,
                    "Judged once the engine is warm.",
                    "Se evalúa cuando el motor esté caliente.",
                ),
            )
        } else if total.abs() > 15.0 {
            (
                Status::Problem,
                if total > 0.0 {
                    tr!(
                        lang,
                        "Running lean: adding {total:.0} % fuel.",
                        "Mezcla pobre: agrega {total:.0} % de combustible."
                    )
                } else {
                    tr!(
                        lang,
                        "Running rich: removing {:.0} % fuel.",
                        "Mezcla rica: quita {:.0} % de combustible.",
                        total.abs()
                    )
                },
            )
        } else if total.abs() > 10.0 {
            (
                Status::Attention,
                if total > 0.0 {
                    tr!(
                        lang,
                        "A little lean: adding {total:.0} % fuel.",
                        "Algo pobre: agrega {total:.0} % de combustible."
                    )
                } else {
                    tr!(
                        lang,
                        "A little rich: removing {:.0} % fuel.",
                        "Algo rica: quita {:.0} % de combustible.",
                        total.abs()
                    )
                },
            )
        } else {
            (
                Status::Good,
                pick(
                    lang,
                    "Air and fuel are well matched.",
                    "El aire y el combustible están bien equilibrados.",
                ),
            )
        };
        let mut it = item(format!("fuel_trim_{bank}"), Area::Engine, title, status);
        it.value = Some(format!("{total:+.1} %"));
        it.headline = headline;
        it.what = what;
        if matches!(status, Status::Attention | Status::Problem) {
            let code = if total > 0.0 { lean } else { rich };
            let (g, fixes) = guide_for(code, lang);
            it.problem = Some(if total > 0.0 {
                pick(
                    lang,
                    "The engine sees more air than it measured, so it keeps adding fuel. Past about 25 % it gives up and sets a lean code.",
                    "El motor ve más aire del que midió, así que sigue agregando combustible. Pasado un 25 % se rinde y guarda un código de mezcla pobre.",
                )
            } else {
                pick(
                    lang,
                    "The engine is getting more fuel than it planned, so it keeps taking fuel away. Rich running fouls plugs and the catalytic converter.",
                    "El motor recibe más combustible del planeado, así que sigue quitando. Una mezcla rica ensucia las bujías y el catalizador.",
                )
            });
            it.fix = fixes;
            it.guide = Some(g);
        }
        it.facts = facts;
        out.push(it);
    }

    // Values worth knowing that one reading cannot judge.
    let info: &[(u8, &str, &str, &str, &str)] = &[
        (0x0C, "Idle speed", "Ralentí",
         "How fast the crankshaft turns. A warm engine idles steadily, usually 600-800 rpm; a hunting or rough idle points to air leaks, misfires or a dirty throttle.",
         "Qué tan rápido gira el cigüeñal. Un motor caliente mantiene un ralentí estable, normalmente 600-800 rpm; un ralentí que sube y baja o es irregular apunta a fugas de aire, fallas de encendido o un acelerador sucio."),
        (0x04, "Engine load", "Carga del motor",
         "How hard the engine is working compared with its maximum. At a warm idle it is usually 15-35 %.",
         "Qué tanto trabaja el motor comparado con su máximo. En ralentí caliente suele estar entre 15 y 35 %."),
        (0x10, "Air flow", "Flujo de aire",
         "Grams of air per second measured by the air-flow sensor. At idle, roughly 1-1.5 g/s per litre of engine size; low readings under load suggest a dirty sensor.",
         "Gramos de aire por segundo que mide el sensor de flujo. En ralentí, más o menos 1-1,5 g/s por litro de cilindrada; valores bajos con carga sugieren un sensor sucio."),
        (0x0B, "Intake pressure", "Presión de admisión",
         "Pressure in the intake manifold. Around 30-40 kPa at idle on most engines. Engines that steer air with the valves (BMW Valvetronic) read near outside pressure by design.",
         "Presión en el múltiple de admisión. Unos 30-40 kPa en ralentí en la mayoría de los motores. Los motores que regulan el aire con las válvulas (BMW Valvetronic) marcan casi la presión exterior por diseño."),
        (0x0E, "Ignition timing", "Avance de encendido",
         "How early the spark fires before the piston reaches the top. The engine module pulls it back when it hears knock.",
         "Qué tan antes salta la chispa antes de que el pistón llegue arriba. El módulo del motor lo retrasa cuando detecta cascabeleo."),
        (0x14, "Upstream oxygen sensor", "Sensor de oxígeno delantero",
         "Measures oxygen in the exhaust before the catalytic converter so the engine can trim fuel. A healthy one swings quickly between about 0.1 and 0.9 V; one reading can't show that, the Repair Copilot's catalyst check watches it.",
         "Mide el oxígeno en el escape antes del catalizador para que el motor ajuste el combustible. Uno sano oscila rápido entre 0,1 y 0,9 V; una sola lectura no lo muestra, la prueba de catalizador del Copiloto lo vigila."),
        (0x15, "Downstream oxygen sensor", "Sensor de oxígeno trasero",
         "Sits after the catalytic converter. A steady signal means the converter is storing oxygen and cleaning the exhaust; if it copies the upstream sensor, the converter is worn.",
         "Va después del catalizador. Una señal estable significa que el catalizador guarda oxígeno y limpia el escape; si copia al sensor delantero, el catalizador está gastado."),
        (0x11, "Throttle", "Acelerador",
         "How far the throttle plate is open. On Valvetronic BMWs it stays mostly open because the valves control airflow.",
         "Qué tan abierta está la mariposa. En los BMW con Valvetronic se queda casi abierta porque las válvulas controlan el aire."),
        (0x0F, "Intake air temperature", "Temperatura del aire de admisión",
         "Air temperature entering the engine. Close to outside temperature when cold; much higher at idle is normal heat soak.",
         "Temperatura del aire que entra al motor. Parecida a la exterior en frío; más alta en ralentí es calor acumulado normal."),
        (0x46, "Outside temperature", "Temperatura exterior",
         "The car's ambient air sensor, used by the climate control and some engine strategies.",
         "El sensor de aire exterior del auto, que usan el clima y algunas estrategias del motor."),
        (0x5C, "Oil temperature", "Temperatura del aceite",
         "Oil warms more slowly than coolant. Hard driving before it reaches about 80 °C wears the engine faster.",
         "El aceite se calienta más lento que el refrigerante. Manejar fuerte antes de que llegue a unos 80 °C desgasta más el motor."),
        (0x3C, "Catalyst temperature", "Temperatura del catalizador",
         "The engine module's estimate of the catalytic converter's temperature. It needs a few hundred degrees to clean the exhaust.",
         "La estimación del módulo del motor de la temperatura del catalizador. Necesita unos cientos de grados para limpiar el escape."),
        (0x2F, "Fuel level", "Nivel de combustible",
         "Tank level. The evaporative leak test only runs with the tank between about a quarter and three quarters full.",
         "Nivel del tanque. La prueba de fugas de vapores solo corre con el tanque entre un cuarto y tres cuartos."),
        (0x33, "Barometric pressure", "Presión barométrica",
         "Outside air pressure, about 101 kPa at sea level and lower at altitude. The engine uses it to correct fuel.",
         "Presión del aire exterior, unos 101 kPa al nivel del mar y menos en altura. El motor la usa para corregir el combustible."),
    ];
    let running_only = [0x0C, 0x04, 0x10, 0x0E, 0x14, 0x15, 0x3C];
    for &(pid, en, es, what_en, what_es) in info {
        let Some(v) = i.pid(pid) else { continue };
        if running_only.contains(&pid) && !running {
            continue;
        }
        let unit_value = match pid {
            0x0C => format!("{v:.0} rpm"),
            0x04 | 0x11 | 0x2F => format!("{v:.0} %"),
            0x10 => format!("{v:.1} g/s"),
            0x0B | 0x33 => format!("{v:.0} kPa"),
            0x0E => format!("{v:.1}°"),
            0x14 | 0x15 => format!("{v:.2} V"),
            _ => format!("{v:.0} °C"),
        };
        let mut it = item(
            format!("pid_{pid:02x}"),
            Area::Engine,
            pick(lang, en, es),
            Status::Info,
        );
        it.value = Some(unit_value);
        it.what = pick(lang, what_en, what_es);
        // The first sentence of the explanation makes a good one-liner.
        it.headline = it
            .what
            .split_once(". ")
            .map_or(it.what.clone(), |(first, _)| format!("{first}."));
        if pid == 0x5C && v > 130.0 {
            it.status = Status::Attention;
            it.headline = pick(lang, "Oil is very hot.", "El aceite está muy caliente.");
            it.problem = Some(pick(
                lang,
                "Above about 130 °C oil starts to thin and break down.",
                "Por encima de unos 130 °C el aceite empieza a perder viscosidad y a degradarse.",
            ));
            it.fix = vec![pick(
                lang,
                "Check the oil level and the cooling system, and ease off until it comes down.",
                "Revisa el nivel de aceite y el sistema de enfriamiento, y maneja suave hasta que baje.",
            )];
        }
        if pid == 0x15 && running {
            let (g, _) = guide_for("P0420", lang);
            it.guide = Some(g);
        }
        out.push(it);
    }

    if !running && i.engine_running.is_some() {
        let mut it = item(
            "engine_off",
            Area::Engine,
            pick(lang, "Running checks", "Pruebas con el motor encendido"),
            Status::NotRead,
        );
        it.headline = pick(
            lang,
            "The engine was off, so fuel mixture, idle and charging were not judged.",
            "El motor estaba apagado, así que no se evaluaron la mezcla, el ralentí ni la carga.",
        );
        it.what = pick(
            lang,
            "Some readings only mean something with the engine running and warm: fuel trims, idle speed, oxygen sensors and the alternator's charging voltage.",
            "Algunas lecturas solo significan algo con el motor encendido y caliente: ajustes de combustible, ralentí, sensores de oxígeno y el voltaje de carga del alternador.",
        );
        it.fix = vec![pick(
            lang,
            "Start the engine, let it idle in Park until warm, and run the full scan again.",
            "Enciende el motor, déjalo en ralentí en Park hasta que se caliente y vuelve a hacer el escaneo completo.",
        )];
        out.push(it);
    }
}

// ---- emissions ---------------------------------------------------------------

/// Plain Spanish names and explanations for readiness monitors.
fn monitor_text(id: &str, lang: Lang, name: &str, explain: &str) -> (String, String) {
    if lang == Lang::En {
        return (name.to_string(), explain.to_string());
    }
    let (n, e) = match id {
        "misfire" => ("Fallas de encendido", "Vigila la velocidad del cigüeñal para detectar cilindros que fallan. Corre todo el tiempo."),
        "fuel" => ("Sistema de combustible", "Comprueba que los ajustes de combustible se mantengan dentro de los límites. Corre todo el tiempo."),
        "components" => ("Componentes generales", "Revisa sensores y actuadores por circuitos abiertos o en corto. Corre todo el tiempo."),
        "catalyst" => ("Catalizador", "Compara los sensores de oxígeno antes y después del catalizador para juzgar cuánto oxígeno guarda."),
        "heated_catalyst" => ("Catalizador calefactado", "Revisa el calefactor del catalizador, en los autos que lo tienen."),
        "evap" => ("Sistema de vapores (EVAP)", "Prueba fugas en el sistema de vapores del tanque. Suele ser el último en completarse: necesita un arranque en frío y el tanque a medias."),
        "secondary_air" => ("Aire secundario", "Revisa la bomba que sopla aire al escape en los arranques en frío."),
        "ac_refrigerant" => ("Refrigerante del A/C", "Obsoleto; casi nunca está soportado."),
        "o2_sensor" => ("Sensores de oxígeno", "Revisa qué tan rápido cambian los sensores de oxígeno."),
        "o2_heater" => ("Calefactores de sensores de oxígeno", "Revisa los calefactores que llevan los sensores de oxígeno a su temperatura."),
        "egr" => ("EGR / VVT", "Revisa la recirculación de gases de escape, o la distribución variable en autos sin EGR."),
        _ => return (name.to_string(), explain.to_string()),
    };
    (n.to_string(), e.to_string())
}

/// How to get an incomplete monitor to run, in general terms.
fn drive_hint(id: &str, lang: Lang) -> String {
    match id {
        "evap" => pick(
            lang,
            "Needs a cold start after the car sat overnight, with the tank between a quarter and three quarters full, then a normal drive.",
            "Necesita un arranque en frío después de que el auto pase la noche quieto, con el tanque entre un cuarto y tres cuartos, y luego un viaje normal.",
        ),
        "catalyst" => pick(
            lang,
            "Needs several minutes of steady cruising at 70-90 km/h (45-55 mph) once the engine is warm.",
            "Necesita varios minutos a velocidad constante de 70-90 km/h (45-55 mph) con el motor caliente.",
        ),
        "o2_sensor" | "o2_heater" => pick(
            lang,
            "Needs a warm engine, a steady cruise, then a coast-down with your foot off the pedal, and a minute or two at idle.",
            "Necesita el motor caliente, un tramo a velocidad constante, luego soltar el acelerador para desacelerar y un minuto o dos en ralentí.",
        ),
        "egr" => pick(
            lang,
            "Runs during gentle deceleration from cruising speed with the engine warm.",
            "Corre al desacelerar suavemente desde velocidad de crucero con el motor caliente.",
        ),
        "secondary_air" => pick(
            lang,
            "Runs only right after a cold start: let the car sit a few hours, then start it.",
            "Corre solo justo después de un arranque en frío: deja el auto quieto unas horas y luego enciéndelo.",
        ),
        _ => pick(
            lang,
            "Usually completes after a few days of mixed city and highway driving.",
            "Normalmente se completa después de unos días de manejo mixto en ciudad y carretera.",
        ),
    }
}

/// Below these, the codes were cleared recently.
const RECENT_KM: f64 = 160.0;
const RECENT_WARMUPS: f64 = 10.0;

fn emissions(i: &Input, lang: Lang, out: &mut Vec<Item>) {
    let Some(r) = &i.readiness else {
        let mut it = item(
            "readiness",
            Area::Emissions,
            pick(lang, "Emissions readiness", "Preparación de emisiones"),
            Status::NotRead,
        );
        it.headline = pick(lang, "Could not be read.", "No se pudo leer.");
        it.what = readiness_what(lang);
        out.push(it);
        return;
    };
    let incomplete: Vec<_> = r
        .monitors
        .iter()
        .filter(|m| m.supported && !m.complete)
        .collect();
    let supported = r.monitors.iter().filter(|m| m.supported).count();

    // The verdict an inspection station would give.
    // With the light on, the light itself is the problem to fix; readiness
    // only follows from it, so it is flagged but not counted twice.
    let status = if r.mil_on || incomplete.len() >= 2 {
        Status::Attention
    } else {
        Status::Good
    };
    let mut it = item(
        "readiness",
        Area::Emissions,
        pick(
            lang,
            "Ready for an emissions test?",
            "¿Listo para la prueba de emisiones?",
        ),
        status,
    );
    it.value = Some(tr!(
        lang,
        "{} of {supported} complete",
        "{} de {supported} completos",
        supported - incomplete.len()
    ));
    it.what = readiness_what(lang);
    it.headline = if r.mil_on {
        pick(
            lang,
            "Not ready: the check-engine light is on.",
            "No está listo: la luz de revisar motor está encendida.",
        )
    } else if incomplete.is_empty() {
        pick(
            lang,
            "Ready: every self-test has finished.",
            "Listo: todas las autopruebas terminaron.",
        )
    } else if incomplete.len() == 1 {
        pick(
            lang,
            "Ready in most places: one self-test is still running, which most stations allow.",
            "Listo en la mayoría de los lugares: una autoprueba sigue pendiente, lo que casi todas las estaciones permiten.",
        )
    } else {
        tr!(
            lang,
            "Not ready yet: {} self-tests haven't finished.",
            "Aún no está listo: {} autopruebas no han terminado.",
            incomplete.len()
        )
    };
    if status != Status::Good {
        it.problem = Some(if r.mil_on {
            pick(
                lang,
                "Stations fail any car with the light on, whatever the monitors say.",
                "Las estaciones rechazan cualquier auto con la luz encendida, sin importar los monitores.",
            )
        } else {
            pick(
                lang,
                "The self-tests reset when codes are cleared or the battery is disconnected, and need some driving to finish. Most stations allow one incomplete (two on older cars).",
                "Las autopruebas se reinician al borrar códigos o desconectar la batería y necesitan algo de manejo para terminar. Casi todas las estaciones permiten una incompleta (dos en autos más viejos).",
            )
        });
        it.fix = if r.mil_on {
            vec![pick(
                lang,
                "Fix the engine codes first; the monitors complete on their own afterwards.",
                "Arregla primero los códigos del motor; los monitores se completan solos después.",
            )]
        } else {
            vec![pick(
                lang,
                "Drive normally for a few days with a mix of city and highway, then scan again. Each unfinished test below says what it needs.",
                "Maneja normalmente unos días combinando ciudad y carretera, y vuelve a escanear. Cada prueba pendiente de abajo dice lo que necesita.",
            )]
        };
    }
    out.push(it);

    for m in r.monitors.iter().filter(|m| m.supported) {
        let (name, explain) = monitor_text(m.id, lang, m.name, m.explain);
        let mut it = item(
            format!("monitor_{}", m.id),
            Area::Emissions,
            name,
            if m.complete {
                Status::Good
            } else {
                Status::Attention
            },
        );
        it.what = explain;
        if m.complete {
            it.value = Some(pick(lang, "Complete", "Completo"));
            it.headline = pick(
                lang,
                "Self-test finished and passed.",
                "Autoprueba terminada y aprobada.",
            );
        } else {
            it.value = Some(pick(lang, "Not finished", "Pendiente"));
            it.headline = pick(
                lang,
                "Hasn't finished since the last reset.",
                "No ha terminado desde el último reinicio.",
            );
            it.problem = Some(pick(
                lang,
                "Not a fault by itself: the test simply hasn't had the right conditions yet.",
                "No es una falla en sí: la prueba todavía no ha tenido las condiciones adecuadas.",
            ));
            it.fix = vec![drive_hint(m.id, lang)];
        }
        out.push(it);
    }

    // Were codes cleared recently?
    let km = i.pid(0x31);
    let warmups = i.pid(0x30);
    if km.is_some() || warmups.is_some() {
        let recent =
            km.is_some_and(|k| k < RECENT_KM) || warmups.is_some_and(|w| w < RECENT_WARMUPS);
        let resets = incomplete.len() >= 2;
        let status = match (recent, resets) {
            (true, true) => Status::Problem,
            (true, false) => Status::Attention,
            _ => Status::Good,
        };
        let mut it = item(
            "cleared",
            Area::Emissions,
            pick(
                lang,
                "Codes cleared recently?",
                "¿Códigos borrados hace poco?",
            ),
            status,
        );
        if let Some(k) = km {
            it.value = Some(format!("{} km", whole(k)));
            it.facts.push(fact(
                pick(
                    lang,
                    "Distance since last clear",
                    "Distancia desde el último borrado",
                ),
                format!("{} km · {} mi", whole(k), whole(k * 0.621_371)),
            ));
        }
        if let Some(w) = warmups {
            it.facts.push(fact(
                pick(
                    lang,
                    "Warm-ups since last clear",
                    "Calentamientos desde el último borrado",
                ),
                format!("{w:.0}"),
            ));
        }
        it.what = pick(
            lang,
            "The car counts the distance and engine warm-ups since codes were last cleared. Clearing codes hides faults for a while, so on a car you are buying this is the tell.",
            "El auto cuenta la distancia y los calentamientos del motor desde el último borrado de códigos. Borrarlos oculta fallas por un tiempo, así que en un auto que vas a comprar esto lo delata.",
        );
        match status {
            Status::Problem => {
                it.headline = pick(
                    lang,
                    "Cleared very recently, and the self-tests haven't finished since.",
                    "Se borraron hace muy poco y las autopruebas no han terminado desde entonces.",
                );
                it.problem = Some(pick(
                    lang,
                    "This pattern is what a car looks like right after someone cleared its codes, for a repair or to hide a fault.",
                    "Así se ve un auto justo después de que alguien borró sus códigos, por una reparación o para ocultar una falla.",
                ));
                it.fix = vec![pick(
                    lang,
                    "Ask what was fixed. Drive it for a week and scan again: a hidden fault comes back.",
                    "Pregunta qué se arregló. Manéjalo una semana y vuelve a escanear: una falla oculta regresa.",
                )];
            }
            Status::Attention => {
                it.headline = pick(
                    lang,
                    "Cleared recently. Normal after a repair.",
                    "Se borraron hace poco. Es normal después de una reparación.",
                );
                it.problem = Some(pick(
                    lang,
                    "Codes were erased not long ago.",
                    "Los códigos se borraron hace poco.",
                ));
                it.fix = vec![pick(
                    lang,
                    "Ask what was repaired, and scan again after a few days of driving.",
                    "Pregunta qué se reparó y vuelve a escanear después de unos días de manejo.",
                )];
            }
            _ => {
                it.headline = pick(
                    lang,
                    "No sign of a recent clear.",
                    "No hay señales de un borrado reciente.",
                );
            }
        }
        out.push(it);
    }
}

fn readiness_what(lang: Lang) -> String {
    pick(
        lang,
        "Readiness monitors are the engine's own emissions self-tests. An inspection station plugs in and checks that they have finished and the light is off, instead of measuring the exhaust.",
        "Los monitores de preparación son las autopruebas de emisiones del propio motor. Una estación de inspección se conecta y comprueba que hayan terminado y que la luz esté apagada, en lugar de medir el escape.",
    )
}

// ---- electrical --------------------------------------------------------------

fn battery(i: &Input, lang: Lang, out: &mut Vec<Item>) {
    let title = pick(lang, "Battery and charging", "Batería y carga");
    let what = pick(
        lang,
        "The battery starts the car and steadies the voltage every module runs on; the alternator recharges it while the engine runs. Weak voltage causes strange faults across many modules at once.",
        "La batería arranca el auto y estabiliza el voltaje con el que funcionan todos los módulos; el alternador la recarga con el motor encendido. Un voltaje bajo causa fallas raras en muchos módulos a la vez.",
    );
    let Some(v) = i.battery_v else {
        let mut it = item("battery", Area::Electrical, title, Status::NotRead);
        it.headline = pick(
            lang,
            "Voltage could not be read.",
            "No se pudo leer el voltaje.",
        );
        it.what = what;
        out.push(it);
        return;
    };
    let running = i.engine_running == Some(true);
    let (status, headline, problem, fix, guide): (
        Status,
        String,
        Option<String>,
        Vec<String>,
        Option<Guide>,
    ) = if running {
        if v < 13.2 {
            let (g, mut f) = guide_for("P0562", lang);
            f.insert(0, pick(lang, "Check the battery terminals and the alternator's main cable for corrosion or looseness first.", "Revisa primero que los bornes de la batería y el cable principal del alternador no estén sulfatados ni flojos."));
            (Status::Problem, pick(lang, "Not charging properly.", "No está cargando bien."),
             Some(pick(lang, "With the engine running the alternator should hold about 13.5-14.8 V. Below 13.2 V the battery is slowly draining while you drive.", "Con el motor encendido el alternador debería mantener unos 13,5-14,8 V. Por debajo de 13,2 V la batería se descarga poco a poco mientras manejas.")), f, Some(g))
        } else if v < 13.5 {
            (Status::Attention, pick(lang, "Charging on the low side.", "Carga algo baja."),
             Some(pick(lang, "A little under the usual 13.5-14.8 V. Some cars lower the voltage on purpose when the battery is full, so recheck with the headlights and blower on.", "Un poco por debajo de los 13,5-14,8 V habituales. Algunos autos bajan el voltaje a propósito cuando la batería está llena, así que vuelve a medir con las luces y el ventilador encendidos.")),
             vec![pick(lang, "Turn on the headlights, blower and rear defrost and scan again: a healthy alternator holds above 13.5 V.", "Enciende las luces, el ventilador y el desempañador trasero y vuelve a escanear: un alternador sano se mantiene arriba de 13,5 V.")], None)
        } else if v > 15.0 {
            (Status::Problem, pick(lang, "Overcharging.", "Sobrecarga."),
             Some(pick(lang, "Above 15 V the battery boils and electronics can be damaged.", "Por encima de 15 V la batería hierve y la electrónica se puede dañar.")),
             vec![pick(lang, "Have the alternator's voltage regulator tested; it usually fails as part of the alternator.", "Haz probar el regulador de voltaje del alternador; normalmente falla junto con el alternador.")], None)
        } else {
            (
                Status::Good,
                pick(
                    lang,
                    "The alternator is charging normally.",
                    "El alternador está cargando con normalidad.",
                ),
                None,
                vec![],
                None,
            )
        }
    } else if v < 12.2 {
        (Status::Problem, pick(lang, "Battery is low.", "La batería está baja."),
         Some(pick(lang, "A rested battery below 12.2 V is less than half charged. It may not start the car, and low voltage makes modules log faults.", "Una batería en reposo por debajo de 12,2 V tiene menos de media carga. Puede que no arranque el auto, y el voltaje bajo hace que los módulos guarden fallas.")),
         battery_fix(i, lang), None)
    } else if v < 12.4 {
        (
            Status::Attention,
            pick(
                lang,
                "Battery only partly charged.",
                "La batería está solo parcialmente cargada.",
            ),
            Some(pick(
                lang,
                "A fully charged battery rests at about 12.6 V.",
                "Una batería con carga completa descansa en unos 12,6 V.",
            )),
            battery_fix(i, lang),
            None,
        )
    } else {
        (
            Status::Good,
            pick(lang, "Battery well charged.", "Batería bien cargada."),
            None,
            vec![],
            None,
        )
    };
    let mut it = item("battery", Area::Electrical, title, status);
    it.value = Some(format!("{v:.1} V"));
    it.headline = headline;
    it.what = what;
    it.problem = problem;
    it.fix = fix;
    it.guide = guide;
    it.facts.push(fact(
        pick(lang, "Measured with", "Medido con"),
        if running {
            pick(lang, "engine running", "motor encendido")
        } else {
            pick(lang, "engine off", "motor apagado")
        },
    ));
    out.push(it);
}

fn battery_fix(i: &Input, lang: Lang) -> Vec<String> {
    let mut f = vec![
        pick(
            lang,
            "Charge it with a proper charger, let it rest an hour, and measure again. If it falls below 12.4 V again, have it load-tested.",
            "Cárgala con un cargador adecuado, déjala reposar una hora y mide de nuevo. Si vuelve a bajar de 12,4 V, haz que le hagan una prueba de carga.",
        ),
        pick(
            lang,
            "If a new battery drains overnight, look for something staying awake (a parasitic drain).",
            "Si una batería nueva se descarga en una noche, busca algo que se quede encendido (un consumo parásito).",
        ),
    ];
    if i.is_bmw {
        f.push(pick(
            lang,
            "On a BMW, register a new battery in Special Functions so the car charges it correctly.",
            "En un BMW, registra la batería nueva en Funciones especiales para que el auto la cargue correctamente.",
        ));
    }
    f
}

// ---- modules -----------------------------------------------------------------

fn modules(i: &Input, lang: Lang, out: &mut Vec<Item>) {
    let blocked = i.is_bmw && i.adapter_extended == Some(false);
    let engine_codes: Vec<&str> = i
        .stored
        .iter()
        .flatten()
        .chain(&i.pending)
        .chain(&i.permanent)
        .map(|c| c.code.as_str())
        .collect();
    let mut absent = Vec::new();
    for m in &i.modules {
        if !m.present {
            absent.push(m.name.clone());
            continue;
        }
        let active = m
            .codes
            .iter()
            .filter(|c| c.state == CodeState::Active)
            .count();
        // The engine module's UDS memory repeats the OBD-II codes already
        // shown above; don't list the same fault twice.
        let only_engine_codes = !m.codes.is_empty()
            && m.codes.iter().all(|c| {
                let base = c.code.get(..5).unwrap_or(&c.code);
                engine_codes.contains(&base)
            });
        let status = if m.codes.is_empty() {
            Status::Good
        } else if only_engine_codes {
            Status::Info
        } else if active > 0 {
            Status::Problem
        } else {
            Status::Attention
        };
        let mut it = item(
            format!("module_{}", m.name),
            Area::Modules,
            m.name.clone(),
            status,
        );
        it.module = Some(m.name.clone());
        it.value = Some(if m.codes.is_empty() {
            pick(lang, "No faults", "Sin fallas")
        } else {
            tr!(
                lang,
                "{} fault{}",
                "{} falla{}",
                m.codes.len(),
                if m.codes.len() == 1 { "" } else { "s" }
            )
        });
        it.what = format!("{}. {}", m.description, m.role);
        it.headline = if m.codes.is_empty() {
            pick(
                lang,
                "Answered with nothing stored.",
                "Respondió sin nada guardado.",
            )
        } else if only_engine_codes {
            pick(
                lang,
                "Holds the same faults as the engine codes above.",
                "Tiene las mismas fallas que los códigos del motor de arriba.",
            )
        } else if active > 0 {
            tr!(
                lang,
                "{active} failing right now.",
                "{active} fallando en este momento."
            )
        } else {
            pick(
                lang,
                "Faults in memory, none failing right now.",
                "Fallas en memoria, ninguna activa ahora.",
            )
        };
        if !m.codes.is_empty() {
            it.facts = m
                .codes
                .iter()
                .map(|c| {
                    let state = match c.state {
                        CodeState::Active => pick(lang, "active", "activa"),
                        CodeState::Pending => pick(lang, "pending", "pendiente"),
                        CodeState::Stored => pick(lang, "stored", "guardada"),
                    };
                    let text = c.description.clone().unwrap_or_else(|| {
                        pick(
                            lang,
                            "fault text not in the database yet",
                            "texto de la falla aún no está en la base de datos",
                        )
                    });
                    fact(c.code.clone(), format!("{text} ({state})"))
                })
                .collect();
        }
        if !m.codes.is_empty() && !only_engine_codes {
            it.problem = Some(if active > 0 {
                pick(
                    lang,
                    "An active fault is happening now, so the system it belongs to may not be working fully.",
                    "Una falla activa está ocurriendo ahora, así que el sistema al que pertenece puede no estar funcionando del todo.",
                )
            } else {
                pick(
                    lang,
                    "Stored faults happened earlier. Many come from a low battery or a past repair; read them before clearing.",
                    "Las fallas guardadas ocurrieron antes. Muchas vienen de una batería baja o de una reparación anterior; léelas antes de borrarlas.",
                )
            });
            let first = m
                .codes
                .iter()
                .find(|c| c.state == CodeState::Active)
                .unwrap_or(&m.codes[0]);
            let (g, fixes) = guide_for(&first.code, lang);
            it.fix = fixes;
            if active == 0 {
                it.fix.push(pick(
                    lang,
                    "If the battery was low or the car was recently repaired, clear the faults, drive, and scan again: only real faults come back.",
                    "Si la batería estuvo baja o el auto se reparó hace poco, borra las fallas, maneja y vuelve a escanear: solo regresan las fallas reales.",
                ));
            }
            it.guide = Some(g);
        }
        out.push(it);
    }
    if !absent.is_empty() && !blocked {
        let mut it = item(
            "modules_silent",
            Area::Modules,
            pick(
                lang,
                "Modules that didn't answer",
                "Módulos que no respondieron",
            ),
            Status::Info,
        );
        it.value = Some(absent.len().to_string());
        it.headline = absent.join(", ");
        it.what = pick(
            lang,
            "Not every car has every module: options vary by trim and year. A module that is fitted but silent can also be asleep or have lost power.",
            "No todos los autos tienen todos los módulos: los equipos varían según versión y año. Un módulo instalado que no responde también puede estar dormido o sin alimentación.",
        );
        it.fix = vec![pick(
            lang,
            "If the car has that feature and it doesn't work, check the module's fuse and scan again with the ignition on.",
            "Si el auto tiene esa función y no funciona, revisa el fusible del módulo y vuelve a escanear con el encendido en ON.",
        )];
        out.push(it);
    }
    if i.modules_not_scanned > 0 {
        let mut it = item(
            "modules_unverified",
            Area::Modules,
            pick(lang, "Modules not asked", "Módulos no consultados"),
            Status::NotRead,
        );
        it.value = Some(i.modules_not_scanned.to_string());
        it.headline = pick(
            lang,
            "Their addresses aren't confirmed for a real car of this kind yet, so the app doesn't guess.",
            "Sus direcciones aún no están confirmadas para un auto real de este tipo, así que la app no adivina.",
        );
        it.what = pick(
            lang,
            "Asking the wrong address wastes time and can confuse other modules, so only confirmed addresses are scanned.",
            "Consultar una dirección equivocada pierde tiempo y puede confundir a otros módulos, así que solo se escanean direcciones confirmadas.",
        );
        out.push(it);
    }
}

// ---- report ------------------------------------------------------------------

/// Turns the scan's readings into the results page.
pub fn build(i: &Input, lang: Lang) -> Report {
    let mut items = Vec::new();
    engine_codes(i, lang, &mut items);
    battery(i, lang, &mut items);
    emissions(i, lang, &mut items);
    modules(i, lang, &mut items);
    live(i, lang, &mut items);
    vehicle(i, lang, &mut items);
    adapter(i, lang, &mut items);

    let mut counts = Counts::default();
    for it in &items {
        match it.status {
            Status::Problem => counts.problem += 1,
            Status::Attention => counts.attention += 1,
            Status::Good => counts.good += 1,
            Status::Info => counts.info += 1,
            Status::NotRead => counts.not_read += 1,
        }
    }
    let overall = items
        .iter()
        .map(|i| i.status)
        .filter(|s| *s != Status::NotRead)
        .max()
        .unwrap_or(Status::NotRead);
    let checked = counts.problem + counts.attention + counts.good + counts.info;
    let title = match overall {
        Status::Problem => pick(lang, "Needs repair", "Necesita reparación"),
        Status::Attention => pick(
            lang,
            "Mostly healthy, a few things to check",
            "Casi todo bien, hay cosas que revisar",
        ),
        Status::Good | Status::Info => pick(lang, "Healthy", "En buen estado"),
        Status::NotRead => pick(lang, "Nothing could be read", "No se pudo leer nada"),
    };
    let summary = match overall {
        Status::NotRead => pick(
            lang,
            "Check the adapter is plugged in and the ignition is on, then scan again.",
            "Revisa que el adaptador esté conectado y el encendido en ON, y vuelve a escanear.",
        ),
        _ => tr!(
            lang,
            "{checked} readings: {} problem{}, {} to check, {} good.",
            "{checked} lecturas: {} problema{}, {} por revisar, {} bien.",
            counts.problem,
            if counts.problem == 1 { "" } else { "s" },
            counts.attention,
            counts.good,
        ),
    };
    let mut tips = Vec::new();
    if i.engine_running != Some(true) {
        tips.push(pick(
            lang,
            "Start the engine and let it warm up to also check fuel mixture, idle and charging.",
            "Enciende el motor y deja que se caliente para revisar también la mezcla, el ralentí y la carga.",
        ));
    }
    if i.is_bmw && i.adapter_extended == Some(false) {
        tips.push(pick(
            lang,
            "An STN-based adapter (OBDLink MX+ or CX) would let the scan reach every BMW module.",
            "Un adaptador con chip STN (OBDLink MX+ o CX) permitiría que el escaneo llegue a todos los módulos BMW.",
        ));
    }
    Report {
        overall,
        title,
        summary,
        counts,
        items,
        tips,
    }
}

/// The live PIDs the scan reads, in the order a technician looks at them.
/// Only those the car says it supports are actually requested.
pub const LIVE_PIDS: &[u8] = &[
    0x0C, 0x0D, 0x05, 0x06, 0x07, 0x08, 0x09, 0x04, 0x0B, 0x10, 0x11, 0x0E, 0x14, 0x15, 0x0F, 0x46,
    0x5C, 0x3C, 0x2F, 0x33, 0x1F, 0x21, 0x30, 0x31, 0x42, 0xA6,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obd2::decode_readiness;

    fn ready() -> Readiness {
        // MIL off, no codes; spark; misfire/fuel/components complete;
        // catalyst, evap, o2, o2 heater, egr supported and complete.
        decode_readiness(&[0x41, 0x01, 0x00, 0x07, 0xE5, 0x00]).unwrap()
    }

    fn healthy() -> Input {
        Input {
            vehicle: "Test car".into(),
            readiness: Some(ready()),
            stored: Some(vec![]),
            battery_v: Some(14.1),
            engine_running: Some(true),
            live: vec![
                (0x05, 92.0),
                (0x06, 1.5),
                (0x07, 2.3),
                (0x0C, 720.0),
                (0x1F, 600.0),
                (0x31, 4200.0),
                (0x30, 140.0),
            ],
            adapter_version: Some("ELM327 v1.5".into()),
            adapter_extended: Some(true),
            modules: vec![ModuleRead {
                name: "DSC".into(),
                description: "Stability control".into(),
                role: "Brakes.".into(),
                system: System::Chassis,
                present: true,
                codes: vec![],
            }],
            ..Default::default()
        }
    }

    fn find<'a>(r: &'a Report, id: &str) -> &'a Item {
        r.items
            .iter()
            .find(|i| i.id == id)
            .unwrap_or_else(|| panic!("no {id}"))
    }

    #[test]
    fn healthy_car_is_good_and_explains_each_part() {
        let r = build(&healthy(), Lang::En);
        assert_eq!(r.overall, Status::Good, "{:#?}", r.items);
        assert_eq!(r.counts.problem + r.counts.attention, 0);
        // Good readings still say what the part does.
        for it in r.items.iter().filter(|i| i.status == Status::Good) {
            assert!(!it.what.is_empty(), "{} has no explanation", it.id);
            assert!(it.fix.is_empty(), "{} is good but has a fix", it.id);
        }
        assert_eq!(find(&r, "fuel_trim_1").status, Status::Good);
        assert_eq!(find(&r, "battery").status, Status::Good);
        assert_eq!(find(&r, "module_DSC").status, Status::Good);
        assert_eq!(find(&r, "module_DSC").what, "Stability control. Brakes.");
    }

    #[test]
    fn bad_readings_say_what_is_wrong_and_how_to_fix_it() {
        let mut i = healthy();
        let mut rd = ready();
        rd.mil_on = true;
        i.readiness = Some(rd);
        i.stored = Some(vec![EngineCode {
            code: "P0171".into(),
            description: Some("System too lean, bank 1".into()),
        }]);
        i.live = vec![(0x05, 95.0), (0x06, 6.0), (0x07, 14.0), (0x1F, 900.0)];
        i.modules[0].codes = vec![ModuleCode {
            code: "5E20".into(),
            description: None,
            state: CodeState::Active,
        }];
        let r = build(&i, Lang::En);
        assert_eq!(r.overall, Status::Problem);
        for it in r
            .items
            .iter()
            .filter(|i| matches!(i.status, Status::Problem | Status::Attention))
        {
            assert!(it.problem.is_some(), "{} has no problem text", it.id);
            assert!(!it.fix.is_empty(), "{} has no fix", it.id);
        }
        let code = find(&r, "code_P0171");
        assert_eq!(code.status, Status::Problem);
        let g = code.guide.as_ref().unwrap();
        assert!(g.playbook);
        assert!(code.fix[0].starts_with("Most likely"));
        let trim = find(&r, "fuel_trim_1");
        assert_eq!(trim.status, Status::Problem);
        assert_eq!(trim.guide.as_ref().unwrap().code, "P0171");
        assert_eq!(find(&r, "module_DSC").status, Status::Problem);
    }

    #[test]
    fn engine_off_does_not_judge_running_values() {
        let mut i = healthy();
        i.engine_running = Some(false);
        i.battery_v = Some(12.6);
        i.live = vec![(0x05, 20.0), (0x06, 0.0), (0x07, 25.0), (0x0C, 0.0)];
        let r = build(&i, Lang::En);
        assert_eq!(find(&r, "fuel_trim_1").status, Status::NotRead);
        assert_eq!(find(&r, "coolant").status, Status::Info);
        assert_eq!(find(&r, "battery").status, Status::Good);
        assert!(r.items.iter().all(|i| i.id != "pid_0c"));
        assert_eq!(find(&r, "engine_off").status, Status::NotRead);
        assert!(!r.tips.is_empty());
    }

    #[test]
    fn pending_code_is_attention_and_permanent_merges() {
        let mut i = healthy();
        i.stored = Some(vec![EngineCode {
            code: "P0420".into(),
            description: None,
        }]);
        i.permanent = vec![EngineCode {
            code: "P0420".into(),
            description: None,
        }];
        i.pending = vec![EngineCode {
            code: "P0300".into(),
            description: None,
        }];
        let r = build(&i, Lang::En);
        assert_eq!(r.items.iter().filter(|i| i.id == "code_P0420").count(), 1);
        assert_eq!(
            find(&r, "code_P0420").value.as_deref(),
            Some("stored · permanent")
        );
        assert_eq!(find(&r, "code_P0300").status, Status::Attention);
    }

    #[test]
    fn recent_clear_with_incomplete_monitors_is_a_problem() {
        let mut i = healthy();
        // catalyst, evap and o2 incomplete
        i.readiness = Some(decode_readiness(&[0x41, 0x01, 0x00, 0x07, 0xE5, 0x25]).unwrap());
        i.live.retain(|(p, _)| *p != 0x31 && *p != 0x30);
        i.live.push((0x31, 38.0));
        i.live.push((0x30, 3.0));
        let r = build(&i, Lang::En);
        assert_eq!(find(&r, "cleared").status, Status::Problem);
        assert_eq!(find(&r, "readiness").status, Status::Attention);
        assert_eq!(find(&r, "monitor_evap").status, Status::Attention);
        assert!(find(&r, "monitor_evap").fix[0].contains("cold start"));
    }

    #[test]
    fn bmw_adapter_without_extended_addressing_is_explained() {
        let mut i = healthy();
        i.is_bmw = true;
        i.adapter_extended = Some(false);
        i.modules[0].present = false;
        let r = build(&i, Lang::En);
        assert_eq!(find(&r, "adapter").status, Status::Problem);
        // Silent modules are not blamed on the car.
        assert!(r.items.iter().all(|i| i.id != "modules_silent"));
    }

    #[test]
    fn engine_module_repeating_obd_codes_is_not_listed_twice() {
        let mut i = healthy();
        i.stored = Some(vec![EngineCode {
            code: "P0420".into(),
            description: None,
        }]);
        i.modules[0].codes = vec![ModuleCode {
            code: "P0420-00".into(),
            description: None,
            state: CodeState::Active,
        }];
        let r = build(&i, Lang::En);
        let m = find(&r, "module_DSC");
        assert_eq!(m.status, Status::Info);
        assert!(m.guide.is_none() && m.fix.is_empty());
        assert_eq!(m.facts.len(), 1);
        assert_eq!(find(&r, "code_P0420").status, Status::Problem);
    }

    #[test]
    fn nothing_read_is_never_a_pass() {
        let r = build(&Input::default(), Lang::En);
        assert_eq!(r.overall, Status::NotRead);
        assert_eq!(find(&r, "mil").status, Status::NotRead);
        assert_eq!(find(&r, "battery").status, Status::NotRead);
    }

    #[test]
    fn spanish_text_is_spanish() {
        let r = build(&healthy(), Lang::Es);
        assert_eq!(find(&r, "battery").title, "Batería y carga");
        assert_eq!(find(&r, "monitor_evap").title, "Sistema de vapores (EVAP)");
        assert!(r.summary.contains("lecturas"));
    }
}
