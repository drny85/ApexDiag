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

/// The language the report is written in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    #[default]
    En,
    Es,
}

/// English or Spanish text, formatted.
macro_rules! tr {
    ($lang:expr, $en:literal, $es:literal $(, $arg:expr)* $(,)?) => {
        match $lang {
            Lang::En => format!($en $(, $arg)*),
            Lang::Es => format!($es $(, $arg)*),
        }
    };
}

/// Singular or plural, in the report's language.
fn pl(lang: Lang, n: usize, en: [&'static str; 2], es: [&'static str; 2]) -> &'static str {
    let i = usize::from(n != 1);
    match lang {
        Lang::En => en[i],
        Lang::Es => es[i],
    }
}

fn pick(lang: Lang, en: &'static str, es: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::Es => es,
    }
}

fn list(codes: &[Code], lang: Lang) -> Vec<String> {
    codes
        .iter()
        .map(|c| match &c.description {
            Some(d) => format!("{}: {d}", c.code),
            None => tr!(
                lang,
                "{}: no description in the app's library yet",
                "{}: todavía sin descripción en la biblioteca de la app",
                c.code
            ),
        })
        .collect()
}

fn engine(r: &Readings, lang: Lang) -> Section {
    let why = pick(
        lang,
        "The engine module stores a code whenever a self-test fails. Stored codes and a lit engine light are the first thing to check on any car.",
        "El módulo del motor guarda un código cada vez que falla una autoprueba. Los códigos guardados y la luz del motor encendida son lo primero que hay que revisar en cualquier auto.",
    );
    let title = pick(
        lang,
        "Engine light and engine codes",
        "Luz y códigos del motor",
    );
    let Some(stored) = &r.stored else {
        return Section {
            id: "engine",
            title,
            grade: Grade::NotChecked,
            headline: tr!(
                lang,
                "The engine codes could not be read.",
                "No se pudieron leer los códigos del motor."
            ),
            details: vec![],
            why,
        };
    };
    let mut details = list(stored, lang);
    let pending = tr!(lang, "pending", "pendiente");
    details.extend(
        list(&r.pending, lang)
            .into_iter()
            .map(|d| format!("{d} ({pending})")),
    );
    let permanent = tr!(
        lang,
        "permanent: clears itself only after the car proves the fix",
        "permanente: solo se borra cuando el auto comprueba la reparación"
    );
    details.extend(
        list(&r.permanent, lang)
            .into_iter()
            .map(|d| format!("{d} ({permanent})")),
    );
    let mil = r.mil_on == Some(true);
    let (grade, headline) = if mil || !stored.is_empty() || !r.permanent.is_empty() {
        let n = stored.len().max(r.permanent.len());
        let codes = match (lang, n) {
            (Lang::En, 1) => "code",
            (Lang::En, _) => "codes",
            (Lang::Es, 1) => "código guardado",
            (Lang::Es, _) => "códigos guardados",
        };
        (
            Grade::Problem,
            match (mil, n) {
                (true, 0) => tr!(
                    lang,
                    "The engine light is on.",
                    "La luz del motor está encendida."
                ),
                (true, n) => tr!(
                    lang,
                    "The engine light is on with {n} stored {codes}.",
                    "La luz del motor está encendida, con {n} {codes}."
                ),
                (false, n) => tr!(
                    lang,
                    "{n} stored {codes} with the engine light off.",
                    "{n} {codes} con la luz del motor apagada."
                ),
            },
        )
    } else if !r.pending.is_empty() {
        let n = r.pending.len();
        let codes = pl(
            lang,
            n,
            ["code", "codes"],
            ["código pendiente", "códigos pendientes"],
        );
        (
            Grade::Attention,
            tr!(
                lang,
                "No stored codes, but {n} pending {codes}: a test failed once and is being watched.",
                "Sin códigos guardados, pero hay {n} {codes}: una prueba falló una vez y está en observación."
            ),
        )
    } else {
        (
            Grade::Good,
            tr!(
                lang,
                "Engine light off and no codes stored or pending.",
                "Luz del motor apagada y sin códigos guardados ni pendientes."
            ),
        )
    };
    Section {
        id: "engine",
        title,
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

fn readiness(r: &Readings, lang: Lang) -> Section {
    let why = pick(
        lang,
        "Readiness monitors are the engine's self-tests for emissions parts. Inspection stations check them, and after codes are cleared they all reset to incomplete.",
        "Los monitores de preparación son las autopruebas del motor para las piezas de emisiones. Los centros de inspección los revisan, y al borrar los códigos todos vuelven a quedar incompletos.",
    );
    let title = pick(lang, "Emissions readiness", "Preparación de emisiones");
    let Some(rd) = &r.readiness else {
        return Section {
            id: "readiness",
            title,
            grade: Grade::NotChecked,
            headline: tr!(
                lang,
                "Readiness could not be read.",
                "No se pudo leer la preparación."
            ),
            details: vec![],
            why,
        };
    };
    let supported: Vec<_> = rd.monitors.iter().filter(|m| m.supported).collect();
    let incomplete: Vec<_> = supported.iter().filter(|m| !m.complete).collect();
    let details = incomplete
        .iter()
        .map(|m| {
            tr!(
                lang,
                "{}: not finished",
                "{}: sin terminar",
                monitor_name(m.name, lang)
            )
        })
        .collect();
    let total = supported.len();
    let (grade, headline) = match incomplete.len() {
        0 => (
            Grade::Good,
            tr!(
                lang,
                "All {total} supported monitors have finished.",
                "Los {total} monitores admitidos terminaron."
            ),
        ),
        1 => (
            Grade::Attention,
            tr!(
                lang,
                "One monitor has not finished. Many inspection programs allow one.",
                "Un monitor no ha terminado. Muchos programas de inspección permiten uno."
            ),
        ),
        n => (
            Grade::Problem,
            tr!(
                lang,
                "{n} monitors have not finished. Many inspection programs allow at most one, so the car may not pass yet.",
                "{n} monitores no han terminado. Muchos programas de inspección permiten como máximo uno, así que puede que el auto todavía no pase."
            ),
        ),
    };
    Section {
        id: "readiness",
        title,
        grade,
        headline,
        details,
        why,
    }
}

fn monitor_name(name: &'static str, lang: Lang) -> &'static str {
    if lang == Lang::En {
        return name;
    }
    match name {
        "Misfire" => "Fallo de encendido",
        "Fuel system" => "Sistema de combustible",
        "Comprehensive components" => "Componentes generales",
        "Catalyst" => "Catalizador",
        "Heated catalyst" => "Catalizador calefactado",
        "Evaporative system" => "Sistema evaporativo (EVAP)",
        "Secondary air" => "Aire secundario",
        "A/C refrigerant" => "Refrigerante del A/C",
        "Oxygen sensors" => "Sensores de oxígeno",
        "Oxygen sensor heaters" => "Calentadores de los sensores de oxígeno",
        "EGR / VVT" => "EGR / VVT",
        other => other,
    }
}

/// Below these, the codes were cleared recently.
const RECENT_KM: f64 = 160.0;
const RECENT_WARMUPS: f64 = 10.0;

fn cleared(r: &Readings, lang: Lang) -> Section {
    let why = pick(
        lang,
        "Clearing codes turns the engine light off for a while, which can hide a fault from a buyer. The car keeps counting the distance and warm-ups since the last clear, and its self-tests reset, so a recent clear still shows.",
        "Borrar los códigos apaga la luz del motor por un tiempo, lo que puede ocultarle una falla al comprador. El auto sigue contando la distancia y los calentamientos desde el último borrado, y sus autopruebas se reinician, así que un borrado reciente se nota.",
    );
    let title = pick(
        lang,
        "Codes cleared recently?",
        "¿Códigos borrados hace poco?",
    );
    let incomplete = r.readiness.as_ref().map(|rd| rd.incomplete());
    if r.km_since_clear.is_none() && r.warmups_since_clear.is_none() {
        return Section {
            id: "cleared",
            title,
            grade: Grade::NotChecked,
            headline: tr!(
                lang,
                "This car does not report when its codes were last cleared.",
                "Este auto no informa cuándo se borraron sus códigos por última vez."
            ),
            details: vec![],
            why,
        };
    }
    let mut details = Vec::new();
    if let Some(km) = r.km_since_clear {
        let (k, mi) = (whole(km), whole(km * 0.621_371));
        details.push(tr!(
            lang,
            "Distance since the last clear: {k} km ({mi} miles)",
            "Distancia desde el último borrado: {k} km ({mi} millas)"
        ));
    }
    if let Some(w) = r.warmups_since_clear {
        details.push(tr!(
            lang,
            "Engine warm-ups since the last clear: {w:.0}",
            "Calentamientos del motor desde el último borrado: {w:.0}"
        ));
    }
    let recent_km = r.km_since_clear.is_some_and(|k| k < RECENT_KM);
    let recent_warm = r.warmups_since_clear.is_some_and(|w| w < RECENT_WARMUPS);
    let resets = incomplete.unwrap_or(0) >= 2;
    let (grade, headline) = if (recent_km || recent_warm) && resets {
        (
            Grade::Problem,
            tr!(
                lang,
                "Codes were cleared very recently, and the self-tests have not finished since. Ask the seller why, and re-check after a week of driving.",
                "Los códigos se borraron hace muy poco y las autopruebas no han terminado desde entonces. Pregúntale al vendedor por qué y vuelve a revisar después de una semana de manejo."
            ),
        )
    } else if recent_km || recent_warm {
        (
            Grade::Attention,
            tr!(
                lang,
                "Codes were cleared recently. That's normal after a repair; ask what was fixed.",
                "Los códigos se borraron hace poco. Es normal después de una reparación; pregunta qué se arregló."
            ),
        )
    } else {
        (
            Grade::Good,
            tr!(
                lang,
                "No sign of a recent clear.",
                "No hay señales de un borrado reciente."
            ),
        )
    };
    Section {
        id: "cleared",
        title,
        grade,
        headline,
        details,
        why,
    }
}

fn battery(r: &Readings, lang: Lang) -> Section {
    let why = pick(
        lang,
        "Weak batteries and failing alternators cause odd electrical faults on modern cars. Resting voltage shows the battery's charge; running voltage shows the alternator is charging.",
        "Una batería débil o un alternador que falla causan fallas eléctricas extrañas en los autos modernos. El voltaje en reposo muestra la carga de la batería; con el motor encendido muestra si el alternador está cargando.",
    );
    let title = pick(lang, "Battery and charging", "Batería y carga");
    let (Some(v), running) = (r.battery_v, r.engine_running) else {
        return Section {
            id: "battery",
            title,
            grade: Grade::NotChecked,
            headline: tr!(
                lang,
                "Voltage could not be read.",
                "No se pudo leer el voltaje."
            ),
            details: vec![],
            why,
        };
    };
    let running = running == Some(true);
    let (grade, headline) = if running {
        match v {
            v if v < 13.2 => (
                Grade::Problem,
                tr!(
                    lang,
                    "{v:.1} V with the engine running: the battery is not being charged properly.",
                    "{v:.1} V con el motor encendido: la batería no se está cargando bien."
                ),
            ),
            v if v < 13.5 => (
                Grade::Attention,
                tr!(
                    lang,
                    "{v:.1} V with the engine running: charging is on the low side.",
                    "{v:.1} V con el motor encendido: la carga está algo baja."
                ),
            ),
            v if v > 15.0 => (
                Grade::Problem,
                tr!(
                    lang,
                    "{v:.1} V with the engine running: that's higher than normal, which can damage the battery.",
                    "{v:.1} V con el motor encendido: es más alto de lo normal y puede dañar la batería."
                ),
            ),
            v => (
                Grade::Good,
                tr!(
                    lang,
                    "{v:.1} V with the engine running: charging normally.",
                    "{v:.1} V con el motor encendido: carga normal."
                ),
            ),
        }
    } else {
        match v {
            v if v < 12.2 => (
                Grade::Problem,
                tr!(
                    lang,
                    "{v:.1} V with the engine off: the battery is low.",
                    "{v:.1} V con el motor apagado: la batería está baja."
                ),
            ),
            v if v < 12.4 => (
                Grade::Attention,
                tr!(
                    lang,
                    "{v:.1} V with the engine off: the battery is only partly charged.",
                    "{v:.1} V con el motor apagado: la batería está solo parcialmente cargada."
                ),
            ),
            v => (
                Grade::Good,
                tr!(
                    lang,
                    "{v:.1} V with the engine off: the battery is well charged.",
                    "{v:.1} V con el motor apagado: la batería está bien cargada."
                ),
            ),
        }
    };
    Section {
        id: "battery",
        title,
        grade,
        headline,
        details: vec![
            if running {
                tr!(
                    lang,
                    "For a complete picture, also check the resting voltage with the engine off.",
                    "Para tener el panorama completo, revisa también el voltaje en reposo con el motor apagado."
                )
            } else {
                tr!(
                    lang,
                    "Start the engine and run the report again to check charging.",
                    "Enciende el motor y vuelve a hacer el informe para revisar la carga."
                )
            },
            tr!(
                lang,
                "A voltage reading is a quick check, not a battery load test.",
                "Una lectura de voltaje es una revisión rápida, no una prueba de carga de la batería."
            ),
        ],
        why,
    }
}

fn modules(r: &Readings, lang: Lang) -> Section {
    let why = pick(
        lang,
        "Every control module (brakes, airbags, body, comfort and more) keeps its own fault memory, which the engine light does not show. A car can look perfect with faults stored here.",
        "Cada módulo de control (frenos, bolsas de aire, carrocería, confort y más) tiene su propia memoria de fallas, que la luz del motor no muestra. Un auto puede verse perfecto y tener fallas guardadas aquí.",
    );
    let title = pick(lang, "All modules", "Todos los módulos");
    if r.modules.is_empty() {
        return Section {
            id: "modules",
            title,
            grade: Grade::NotChecked,
            headline: tr!(
                lang,
                "No modules were scanned.",
                "No se escaneó ningún módulo."
            ),
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
            list(&m.faults, lang)
                .into_iter()
                .map(move |f| format!("{}: {f}", m.name))
        })
        .collect();
    let skipped = r.modules_not_scanned;
    let modules = pl(
        lang,
        skipped,
        ["module was", "modules were"],
        ["módulo", "módulos"],
    );
    if skipped > 0 {
        details.push(tr!(
            lang,
            "{skipped} more {modules} not scanned: their addresses aren't confirmed for this car yet.",
            "{skipped} {modules} más sin escanear: sus direcciones todavía no están confirmadas para este auto."
        ));
    }
    let (n_ans, n_with) = (answered.len(), with_faults.len());
    let stored = pl(
        lang,
        faults,
        ["fault stored", "faults stored"],
        ["falla guardada", "fallas guardadas"],
    );
    let (grade, headline) = if with_faults.is_empty() {
        (
            Grade::Good,
            tr!(
                lang,
                "{n_ans} modules answered with no faults stored.",
                "{n_ans} módulos respondieron sin fallas guardadas."
            ),
        )
    } else if safety {
        (
            Grade::Problem,
            tr!(
                lang,
                "{faults} {stored}, including in a safety system. Have it checked before buying.",
                "{faults} {stored}, incluso en un sistema de seguridad. Haz que lo revisen antes de comprar."
            ),
        )
    } else {
        (
            Grade::Attention,
            tr!(
                lang,
                "{faults} {stored} in {n_with} of {n_ans} modules.",
                "{faults} {stored} en {n_with} de {n_ans} módulos."
            ),
        )
    };
    Section {
        id: "modules",
        title,
        grade,
        headline,
        details,
        why,
    }
}

fn mileage(r: &Readings, lang: Lang) -> Section {
    let why = pick(
        lang,
        "A wound-back odometer hides wear. Several modules record mileage of their own, so on cars where those records are confirmed, the app compares them.",
        "Un odómetro alterado oculta el desgaste. Varios módulos guardan su propio kilometraje, así que en los autos donde esos registros están confirmados, la app los compara.",
    );
    let mut details = Vec::new();
    if let Some(km) = r.odometer_km {
        let (k, mi) = (whole(km), whole(km * 0.621_371));
        details.push(tr!(
            lang,
            "Odometer reported by the engine module: {k} km ({mi} miles)",
            "Odómetro según el módulo del motor: {k} km ({mi} millas)"
        ));
    }
    if let Some(km) = r.km_with_mil.filter(|k| *k > 0.0) {
        let k = whole(km);
        details.push(tr!(
            lang,
            "Driven with the engine light on: {k} km",
            "Recorrido con la luz del motor encendida: {k} km"
        ));
    }
    details.push(tr!(
        lang,
        "The module-by-module mileage comparison needs mileage records confirmed for this car; it isn't available yet, so compare the dashboard with the service history.",
        "La comparación de kilometraje módulo por módulo necesita registros confirmados para este auto; todavía no está disponible, así que compara el tablero con el historial de servicio."
    ));
    Section {
        id: "mileage",
        title: pick(lang, "Mileage", "Kilometraje"),
        grade: Grade::NotChecked,
        headline: match r.odometer_km {
            Some(km) => {
                let k = whole(km);
                tr!(
                    lang,
                    "{k} km reported over OBD-II. Not cross-checked yet.",
                    "{k} km informados por OBD-II. Todavía sin verificación cruzada."
                )
            }
            None => tr!(
                lang,
                "Not cross-checked yet.",
                "Todavía sin verificación cruzada."
            ),
        },
        details,
        why,
    }
}

pub fn grade(r: &Readings, lang: Lang) -> Report {
    let sections = vec![
        engine(r, lang),
        modules(r, lang),
        readiness(r, lang),
        cleared(r, lang),
        battery(r, lang),
        mileage(r, lang),
    ];
    let overall = sections
        .iter()
        .map(|s| s.grade)
        .max()
        .unwrap_or(Grade::NotChecked);
    let count = |g| sections.iter().filter(|s| s.grade == g).count();
    let (p, a) = (count(Grade::Problem), count(Grade::Attention));
    let p_areas = pl(
        lang,
        p,
        ["area has a problem", "areas have a problem"],
        ["área tiene un problema", "áreas tienen un problema"],
    );
    let a_areas = pl(
        lang,
        a,
        ["area is worth", "areas are worth"],
        ["área merece", "áreas merecen"],
    );
    let a_more = pl(lang, a, ["needs", "need"], ["merece", "merecen"]);
    let summary = match overall {
        Grade::Good => tr!(
            lang,
            "Everything the app could check looks good.",
            "Todo lo que la app pudo revisar se ve bien."
        ),
        Grade::Attention => tr!(
            lang,
            "{a} {a_areas} a closer look; nothing serious found.",
            "{a} {a_areas} una revisión más detallada; no se encontró nada grave."
        ),
        Grade::Problem if a > 0 => tr!(
            lang,
            "{p} {p_areas} to sort out, and {a} more {a_more} a closer look.",
            "{p} {p_areas} que resolver, y {a} más {a_more} una revisión detallada."
        ),
        Grade::Problem => tr!(
            lang,
            "{p} {p_areas} to sort out.",
            "{p} {p_areas} que resolver."
        ),
        Grade::NotChecked => tr!(
            lang,
            "The car could not be read.",
            "No se pudo leer el auto."
        ),
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
        let r = grade(&healthy(), Lang::En);
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
        let r = grade(&x, Lang::En);
        assert_eq!(by(&r, "cleared").grade, Grade::Problem);
        assert_eq!(by(&r, "readiness").grade, Grade::Problem);
        assert_eq!(r.overall, Grade::Problem);
        // A recent clear with all tests finished is only worth asking about.
        x.readiness = healthy().readiness;
        assert_eq!(by(&grade(&x, Lang::En), "cleared").grade, Grade::Attention);
    }

    #[test]
    fn grades_codes_battery_and_safety_faults() {
        let mut x = healthy();
        x.pending = vec![code("P0456")];
        assert_eq!(by(&grade(&x, Lang::En), "engine").grade, Grade::Attention);
        x.stored = Some(vec![code("P0171")]);
        x.mil_on = Some(true);
        assert_eq!(by(&grade(&x, Lang::En), "engine").grade, Grade::Problem);
        x.battery_v = Some(12.9);
        assert_eq!(by(&grade(&x, Lang::En), "battery").grade, Grade::Problem);
        x.engine_running = Some(false);
        x.battery_v = Some(12.3);
        assert_eq!(by(&grade(&x, Lang::En), "battery").grade, Grade::Attention);
        x.modules[0].faults.push(code("5E20"));
        assert_eq!(by(&grade(&x, Lang::En), "modules").grade, Grade::Attention);
        x.modules[0].system = System::Safety;
        assert_eq!(by(&grade(&x, Lang::En), "modules").grade, Grade::Problem);
    }

    #[test]
    fn writes_spanish() {
        let r = grade(&healthy(), Lang::Es);
        assert_eq!(r.summary, "Todo lo que la app pudo revisar se ve bien.");
        assert_eq!(by(&r, "battery").title, "Batería y carga");
        assert!(by(&r, "battery").headline.contains("carga normal"));
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
        let r = grade(&Readings::default(), Lang::En);
        assert!(r.sections.iter().all(|s| s.grade == Grade::NotChecked));
        assert_eq!(r.overall, Grade::NotChecked);
    }
}
