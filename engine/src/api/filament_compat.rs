//! Compatibilité haute/basse température des filaments, portage exact de
//! `Print::check_multi_filaments_compatibility` (Print.cpp:1052) et de la
//! table `MaterialType::all()` (MaterialType.cpp) d'Orca 2.4.1, pour
//! l'avertissement/refus avant tranchage (`Print::check_multi_filament_valid`).

use serde::Serialize;

/// `FilamentCompatibilityType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FilamentCompatibility {
    Compatible,
    HighLowMixed,
    InvalidTemperatureRange,
}

/// Messages d'Orca (Print.cpp:1149-1151).
pub const INCOMPATIBLE_TEMP_MSG: &str = "Selected nozzle temperatures are incompatible. Each filament's nozzle temperature must fall within the recommended nozzle temperature range of the other filaments. Otherwise, nozzle clogging or printer damage may occur.";
pub const INVALID_TEMP_RANGE_MSG: &str =
    "Invalid recommended nozzle temperature range. The lower bound must be lower than the upper bound.";
pub const ENABLE_IN_PREFERENCES_MSG: &str = "If you still want to print, you can enable the option in Preferences / Control / Slicing / Remove mixed temperature restriction.";

const DEFAULT_MIN_TEMP: i64 = 190;
const DEFAULT_MAX_TEMP: i64 = 300;

/// (type, température min, température max) — MaterialType.cpp, même ordre.
const MATERIALS: &[(&str, i64, i64)] = &[
    ("ABS", 190, 300),
    ("ABS-CF", 220, 300),
    ("ABS-GF", 240, 280),
    ("ASA", 220, 300),
    ("ASA-CF", 230, 300),
    ("ASA-GF", 240, 300),
    ("ASA-AERO", 240, 280),
    ("BVOH", 190, 240),
    ("CoPE", 190, 240),
    ("EVA", 175, 220),
    ("FLEX", 210, 230),
    ("HIPS", 220, 270),
    ("PA", 235, 280),
    ("PA-CF", 240, 315),
    ("PA-GF", 240, 290),
    ("PA6", 260, 300),
    ("PA6-CF", 230, 300),
    ("PA6-GF", 260, 300),
    ("PA11", 275, 295),
    ("PA11-CF", 275, 295),
    ("PA11-GF", 275, 295),
    ("PA12", 250, 270),
    ("PA12-CF", 250, 300),
    ("PA12-GF", 255, 270),
    ("PAHT", 260, 310),
    ("PAHT-CF", 270, 310),
    ("PAHT-GF", 270, 310),
    ("PC", 240, 300),
    ("PC-ABS", 230, 270),
    ("PC-CF", 270, 295),
    ("PC-PBT", 260, 300),
    ("PCL", 130, 170),
    ("PCTG", 220, 300),
    ("PE", 175, 260),
    ("PE-CF", 175, 260),
    ("PE-GF", 230, 270),
    ("PEI-1010", 370, 430),
    ("PEI-1010-CF", 380, 430),
    ("PEI-1010-GF", 380, 430),
    ("PEI-9085", 350, 390),
    ("PEI-9085-CF", 365, 390),
    ("PEI-9085-GF", 370, 390),
    ("PEEK", 350, 460),
    ("PEEK-CF", 380, 410),
    ("PEEK-GF", 375, 410),
    ("PEKK", 325, 400),
    ("PEKK-CF", 360, 400),
    ("PES", 340, 390),
    ("PET", 200, 290),
    ("PET-CF", 240, 320),
    ("PET-GF", 280, 320),
    ("PETG", 190, 260),
    ("PETG-CF", 230, 290),
    ("PETG-GF", 210, 270),
    ("PHA", 190, 250),
    ("PI", 390, 410),
    ("PLA", 180, 240),
    ("PLA-AERO", 220, 270),
    ("PLA-CF", 190, 250),
    ("POM", 210, 250),
    ("PP", 200, 240),
    ("PP-CF", 210, 250),
    ("PP-GF", 220, 260),
    ("PPA-CF", 260, 300),
    ("PPA-GF", 260, 290),
    ("PPS", 300, 345),
    ("PPS-CF", 295, 350),
    ("PPSU", 360, 420),
    ("PSU", 350, 380),
    ("PVA", 185, 250),
    ("PVB", 190, 250),
    ("PVDF", 245, 265),
    ("SBS", 195, 250),
    ("TPI", 420, 445),
    ("TPU", 175, 260),
];

/// `MaterialType::get_temperature_range` : plage du type, sinon 190–300.
pub fn temperature_range(filament_type: &str) -> (i64, i64) {
    MATERIALS
        .iter()
        .find(|(t, _, _)| *t == filament_type)
        .map(|(_, lo, hi)| (*lo, *hi))
        .unwrap_or((DEFAULT_MIN_TEMP, DEFAULT_MAX_TEMP))
}

/// `Print::check_multi_filaments_compatibility` (bornes 0 = plage du type).
pub fn check_multi_filaments_compatibility(
    filament_types: &[String],
    nozzle_temperatures: &[i64],
    range_lows: &[i64],
    range_highs: &[i64],
) -> FilamentCompatibility {
    let n = filament_types.len();
    if n < 2 {
        return FilamentCompatibility::Compatible;
    }
    let mut temps = vec![0; n];
    let mut lows = vec![0; n];
    let mut highs = vec![0; n];
    for i in 0..n {
        let mut low = range_lows.get(i).copied().unwrap_or(0);
        let mut high = range_highs.get(i).copied().unwrap_or(0);
        if low == 0 || high == 0 {
            let (dlow, dhigh) = temperature_range(&filament_types[i]);
            if low == 0 {
                low = dlow;
            }
            if high == 0 {
                high = dhigh;
            }
        }
        if low >= high {
            return FilamentCompatibility::InvalidTemperatureRange;
        }
        temps[i] = nozzle_temperatures.get(i).copied().unwrap_or(0);
        lows[i] = low;
        highs[i] = high;
    }
    for i in 0..n {
        for j in i + 1..n {
            let i_ok = temps[i] >= lows[j] && temps[i] <= highs[j];
            let j_ok = temps[j] >= lows[i] && temps[j] <= highs[i];
            if !(i_ok && j_ok) {
                return FilamentCompatibility::HighLowMixed;
            }
        }
    }
    FilamentCompatibility::Compatible
}

/// Verdict d'Orca (`check_multi_filament_valid`) : message et caractère
/// bloquant selon la préférence « Remove mixed temperature restriction ».
pub fn verdict(compat: FilamentCompatibility, allow_mixed: bool) -> Option<(String, bool)> {
    match compat {
        FilamentCompatibility::Compatible => None,
        FilamentCompatibility::InvalidTemperatureRange => {
            Some((INVALID_TEMP_RANGE_MSG.into(), true))
        }
        FilamentCompatibility::HighLowMixed if allow_mixed => {
            Some((INCOMPATIBLE_TEMP_MSG.into(), false))
        }
        FilamentCompatibility::HighLowMixed => Some((
            format!("{INCOMPATIBLE_TEMP_MSG} {ENABLE_IN_PREFERENCES_MSG}"),
            true,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn matches_orca_rules() {
        assert_eq!(MATERIALS.len(), 75);
        assert_eq!(temperature_range("PLA"), (180, 240));
        assert_eq!(temperature_range("Unknown"), (190, 300));
        // PLA 220 / PETG 240 : chacune dans la plage de l'autre.
        assert_eq!(
            check_multi_filaments_compatibility(
                &s(&["PLA", "PETG"]),
                &[220, 240],
                &[0, 0],
                &[0, 0]
            ),
            FilamentCompatibility::Compatible
        );
        // PLA 220 / PC 270 : 270 hors 180–240.
        assert_eq!(
            check_multi_filaments_compatibility(&s(&["PLA", "PC"]), &[220, 270], &[0, 0], &[0, 0]),
            FilamentCompatibility::HighLowMixed
        );
        // Plages explicites prioritaires ; plage inversée → invalide.
        assert_eq!(
            check_multi_filaments_compatibility(
                &s(&["PLA", "PC"]),
                &[220, 270],
                &[200, 200],
                &[280, 280]
            ),
            FilamentCompatibility::Compatible
        );
        assert_eq!(
            check_multi_filaments_compatibility(
                &s(&["PLA", "PC"]),
                &[220, 270],
                &[250, 0],
                &[240, 0]
            ),
            FilamentCompatibility::InvalidTemperatureRange
        );
        assert_eq!(
            check_multi_filaments_compatibility(&s(&["PLA"]), &[300], &[0], &[0]),
            FilamentCompatibility::Compatible
        );
        let (msg, blocking) = verdict(FilamentCompatibility::HighLowMixed, false).unwrap();
        assert!(blocking && msg.ends_with("Remove mixed temperature restriction."));
        let (msg, blocking) = verdict(FilamentCompatibility::HighLowMixed, true).unwrap();
        assert!(!blocking && msg == INCOMPATIBLE_TEMP_MSG);
        assert!(
            verdict(FilamentCompatibility::InvalidTemperatureRange, true)
                .unwrap()
                .1
        );
    }
}
