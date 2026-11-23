//! VIN decoding (ISO 3779 / SAE J853): who built the car, where, which
//! model year, and whether the check digit is right.
//!
//! Only the parts that are standard across makers are decoded here. Model
//! and engine come from the vehicle pack or from the car itself.

use serde::Serialize;

use crate::vehicle::Make;

/// Prefix of the simulator's invented VINs.
pub const DEMO_PREFIX: &str = "SIMULATED";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VinInfo {
    pub vin: String,
    /// 17 characters, letters and digits, no I, O or Q.
    pub well_formed: bool,
    /// One of the simulator's invented VINs.
    pub demo: bool,
    /// World manufacturer identifier (first 3 characters).
    pub wmi: String,
    pub region: &'static str,
    pub country: Option<&'static str>,
    pub make: Make,
    /// Brand as built (Lexus and Acura map to the Toyota and Honda packs;
    /// a 1C4 VIN may be a Jeep, Dodge or Chrysler).
    pub manufacturer: Option<&'static str>,
    pub model_year: Option<u16>,
    /// Character 11: the assembly plant code.
    pub plant: Option<char>,
    /// Characters 12-17.
    pub serial: String,
    /// Check digit (character 9). Mandatory for North American cars;
    /// European makers may leave it unused, so a mismatch there is normal.
    pub check_digit_ok: Option<bool>,
    pub check_digit_required: bool,
}

const WMI: &[(&str, Make, &str)] = &[
    ("WBA", Make::Bmw, "BMW"),
    ("WBS", Make::Bmw, "BMW M"),
    ("WBX", Make::Bmw, "BMW"),
    ("WBY", Make::Bmw, "BMW i"),
    ("5UX", Make::Bmw, "BMW (Spartanburg)"),
    ("5UJ", Make::Bmw, "BMW (Spartanburg)"),
    ("5YM", Make::Bmw, "BMW M (Spartanburg)"),
    ("4US", Make::Bmw, "BMW (Spartanburg)"),
    ("WDB", Make::MercedesBenz, "Mercedes-Benz"),
    ("WDC", Make::MercedesBenz, "Mercedes-Benz SUV"),
    ("WDD", Make::MercedesBenz, "Mercedes-Benz"),
    ("WDF", Make::MercedesBenz, "Mercedes-Benz van"),
    ("W1K", Make::MercedesBenz, "Mercedes-Benz"),
    ("W1N", Make::MercedesBenz, "Mercedes-Benz SUV"),
    ("W1V", Make::MercedesBenz, "Mercedes-Benz van"),
    ("4JG", Make::MercedesBenz, "Mercedes-Benz (Alabama)"),
    ("55S", Make::MercedesBenz, "Mercedes-Benz (Alabama)"),
    ("JTH", Make::Toyota, "Lexus"),
    ("JTJ", Make::Toyota, "Lexus"),
    ("2T2", Make::Toyota, "Lexus (Canada)"),
    ("JT", Make::Toyota, "Toyota"),
    ("4T1", Make::Toyota, "Toyota (Kentucky)"),
    ("4T3", Make::Toyota, "Toyota (USA)"),
    ("4T4", Make::Toyota, "Toyota (USA)"),
    ("5TD", Make::Toyota, "Toyota (USA)"),
    ("5TE", Make::Toyota, "Toyota (USA)"),
    ("5TF", Make::Toyota, "Toyota (USA)"),
    ("2T1", Make::Toyota, "Toyota (Canada)"),
    ("2T3", Make::Toyota, "Toyota (Canada)"),
    ("3TM", Make::Toyota, "Toyota (Mexico)"),
    ("SB1", Make::Toyota, "Toyota (UK)"),
    ("NMT", Make::Toyota, "Toyota (Turkey)"),
    ("19U", Make::Honda, "Acura"),
    ("JH4", Make::Honda, "Acura"),
    ("5J8", Make::Honda, "Acura"),
    ("JHM", Make::Honda, "Honda"),
    ("JHL", Make::Honda, "Honda"),
    ("1HG", Make::Honda, "Honda (Ohio)"),
    ("2HG", Make::Honda, "Honda (Canada)"),
    ("2HK", Make::Honda, "Honda (Canada)"),
    ("3HG", Make::Honda, "Honda (Mexico)"),
    ("5FN", Make::Honda, "Honda (Alabama)"),
    ("5J6", Make::Honda, "Honda (Ohio)"),
    ("5KB", Make::Honda, "Honda (USA)"),
    ("19X", Make::Honda, "Honda (USA)"),
    ("SHH", Make::Honda, "Honda (UK)"),
    ("SHS", Make::Honda, "Honda (UK)"),
    ("1J4", Make::Jeep, "Jeep"),
    ("1J8", Make::Jeep, "Jeep"),
    // 1C4 is shared by Jeep, Dodge and Chrysler SUVs and vans. The Jeep
    // pack is a demo layout, so a real car only gets the legislated scan.
    ("1C4", Make::Jeep, "Jeep, Dodge or Chrysler (USA)"),
];

fn region(c: char) -> (&'static str, Option<&'static str>) {
    match c {
        'A'..='H' => ("Africa", None),
        'J' => ("Asia", Some("Japan")),
        'K' => ("Asia", Some("South Korea")),
        'L' => ("Asia", Some("China")),
        'M'..='R' => ("Asia", None),
        'S' => ("Europe", Some("United Kingdom")),
        'V' => ("Europe", Some("France or Spain")),
        'W' => ("Europe", Some("Germany")),
        'T'..='Z' => ("Europe", None),
        '1' | '4' | '5' => ("North America", Some("United States")),
        '2' => ("North America", Some("Canada")),
        '3' => ("North America", Some("Mexico")),
        '6' | '7' => ("Oceania", None),
        '8' | '9' => ("South America", None),
        _ => ("Unknown", None),
    }
}

fn translit(c: char) -> Option<u32> {
    Some(match c {
        '0'..='9' => c as u32 - '0' as u32,
        'A' | 'J' => 1,
        'B' | 'K' | 'S' => 2,
        'C' | 'L' | 'T' => 3,
        'D' | 'M' | 'U' => 4,
        'E' | 'N' | 'V' => 5,
        'F' | 'W' => 6,
        'G' | 'P' | 'X' => 7,
        'H' | 'Y' => 8,
        'R' | 'Z' => 9,
        _ => return None,
    })
}

/// The check digit a VIN should carry at position 9.
pub fn check_digit(vin: &str) -> Option<char> {
    const WEIGHTS: [u32; 17] = [8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2];
    if vin.chars().count() != 17 {
        return None;
    }
    let mut sum = 0;
    for (c, w) in vin.chars().zip(WEIGHTS) {
        sum += translit(c)? * w;
    }
    Some(match sum % 11 {
        10 => 'X',
        r => char::from_digit(r, 10)?,
    })
}

const YEAR_CODES: &str = "ABCDEFGHJKLMNPRSTVWXY123456789";

/// Model year from position 10. Position 7 being a letter means the
/// 2010-2039 cycle for North American cars; otherwise the most recent
/// year that is not in the future.
fn model_year(vin: &[char], current_year: u16) -> Option<u16> {
    let idx = YEAR_CODES.find(vin[9])? as u16;
    let early = 1980 + idx;
    let late = early + 30;
    if vin[0].is_ascii_digit() && ('1'..='5').contains(&vin[0]) {
        return Some(if vin[6].is_ascii_alphabetic() {
            late
        } else {
            early
        });
    }
    Some(if late <= current_year + 1 {
        late
    } else {
        early
    })
}

/// Decodes what can be decoded without a database.
pub fn decode(vin: &str, current_year: u16) -> VinInfo {
    let vin = vin.trim().to_ascii_uppercase();
    let chars: Vec<char> = vin.chars().collect();
    let demo = vin.starts_with(DEMO_PREFIX);
    let well_formed = chars.len() == 17
        && chars
            .iter()
            .all(|c| c.is_ascii_alphanumeric() && !matches!(c, 'I' | 'O' | 'Q'));
    let wmi: String = chars.iter().take(3).collect();
    let (region, country) = chars
        .first()
        .map(|&c| region(c))
        .unwrap_or(("Unknown", None));
    let hit = WMI.iter().find(|(prefix, _, _)| vin.starts_with(prefix));
    let check_digit_required = region == "North America";
    let check_digit_ok = (well_formed).then(|| check_digit(&vin) == Some(chars[8]));
    VinInfo {
        well_formed,
        demo,
        region: if demo { "Demo" } else { region },
        country: if demo { None } else { country },
        make: hit.map(|h| h.1).unwrap_or(Make::Generic),
        manufacturer: hit.map(|h| h.2),
        model_year: (chars.len() == 17 && !demo)
            .then(|| model_year(&chars, current_year))
            .flatten(),
        plant: (chars.len() == 17).then(|| chars[10]),
        serial: chars.iter().skip(11).collect(),
        check_digit_ok,
        check_digit_required,
        wmi,
        vin,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_digit_matches_the_standard_example() {
        // The worked example from 49 CFR 565.
        assert_eq!(check_digit("1M8GDM9AXKP042788"), Some('X'));
        let v = decode("1M8GDM9AXKP042788", 2026);
        assert!(v.well_formed);
        assert_eq!(v.check_digit_ok, Some(true));
        assert!(v.check_digit_required);
        assert_eq!(v.region, "North America");
    }

    #[test]
    fn recognises_makes_and_years() {
        // Spartanburg X5, 2008 (position 7 numeric -> 1980-2009 cycle).
        let mut vin = String::from("5UXFE43508L000123");
        let cd = check_digit(&vin).unwrap();
        vin.replace_range(8..9, &cd.to_string());
        let v = decode(&vin, 2026);
        assert_eq!(v.make, Make::Bmw);
        assert_eq!(v.model_year, Some(2008));
        assert_eq!(v.check_digit_ok, Some(true));
        assert_eq!(v.plant, Some('L'));
        assert_eq!(v.serial, "000123");

        // Alabama-built GLE, 2019.
        let v = decode("4JGFB4KB0KA000001", 2026);
        assert_eq!(v.make, Make::MercedesBenz);
        assert_eq!(v.model_year, Some(2019));

        assert_eq!(decode("4T1B11HK5JU000001", 2026).make, Make::Toyota);
        assert_eq!(decode("4T1B11HK5JU000001", 2026).model_year, Some(2018));
        assert_eq!(
            decode("JTHBA1D20K5000001", 2026).manufacturer,
            Some("Lexus")
        );
        assert_eq!(decode("1HGCV1F30JA000001", 2026).make, Make::Honda);
        assert_eq!(
            decode("19UUB1F50JA000001", 2026).manufacturer,
            Some("Acura")
        );
        let v = decode("1C4HJXDG0KW000001", 2026);
        assert_eq!(v.make, Make::Jeep);
        assert_eq!(v.model_year, Some(2019));
        assert_eq!(decode("1J4FA49S0YP000001", 2026).make, Make::Jeep);
        assert_eq!(decode("WDDZF4JB0KA000001", 2026).region, "Europe");
        assert_eq!(decode("WDDZF4JB0KA000001", 2026).country, Some("Germany"));
    }

    #[test]
    fn flags_bad_and_demo_vins() {
        let v = decode("1HGCV1F30JA00000", 2026);
        assert!(!v.well_formed);
        assert_eq!(v.check_digit_ok, None);
        let v = decode("1HGCV1F3OJA000001", 2026); // letter O
        assert!(!v.well_formed);
        let v = decode("SIMULATED0APEX001", 2026);
        assert!(v.demo);
        assert_eq!(v.make, Make::Generic);
        assert_eq!(v.model_year, None);
        assert_eq!(decode("ZZZ00000000000000", 2026).make, Make::Generic);
    }
}
