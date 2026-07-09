//! Conversion entre la sérialisation texte d'Orca (`opt_serialize`) et
//! `ConfigValue`, pilotée par le registre. Utilisée par l'échange 3MF (T014)
//! et la résolution de presets (T037).

use crate::api::ConfigValue;

use super::{ParamDef, ParamKind};

/// Parse une valeur au format texte Orca selon le type du registre.
/// `None` si la valeur est invalide pour ce type.
pub fn parse_orca_value(def: &ParamDef, s: &str) -> Option<ConfigValue> {
    use ParamKind as K;
    let s = s.trim();
    match def.kind {
        K::Float => parse_float(s).map(ConfigValue::Float),
        K::Int => s.parse().ok().map(ConfigValue::Int),
        K::Bool => parse_bool(s).map(ConfigValue::Bool),
        K::Percent => Some(ConfigValue::Float(parse_float(
            s.strip_suffix('%').unwrap_or(s),
        )?)),
        K::FloatOrPercent => {
            if let Some(pct) = s.strip_suffix('%') {
                Some(ConfigValue::FloatOrPercent {
                    value: parse_float(pct)?,
                    percent: true,
                })
            } else {
                Some(ConfigValue::FloatOrPercent {
                    value: parse_float(s)?,
                    percent: false,
                })
            }
        }
        K::String | K::Enum => Some(ConfigValue::String(unquote(s).to_string())),
        K::Floats | K::Percents | K::FloatsOrPercents => {
            if def.nullable && split_vector(s).any(|t| t == "nil") {
                let v: Option<Vec<Option<f64>>> = split_vector(s)
                    .map(|t| {
                        if t == "nil" {
                            Some(None)
                        } else {
                            parse_float(t.strip_suffix('%').unwrap_or(t)).map(Some)
                        }
                    })
                    .collect();
                v.map(ConfigValue::FloatsNullable)
            } else {
                let v: Option<Vec<f64>> = split_vector(s)
                    .map(|t| parse_float(t.strip_suffix('%').unwrap_or(t)))
                    .collect();
                v.map(ConfigValue::Floats)
            }
        }
        K::Ints => split_vector(s)
            .map(|t| t.parse().ok())
            .collect::<Option<Vec<i64>>>()
            .map(ConfigValue::Ints),
        K::Bools => split_vector(s)
            .map(parse_bool)
            .collect::<Option<Vec<bool>>>()
            .map(ConfigValue::Bools),
        K::Strings | K::Enums => Some(ConfigValue::Strings(
            split_strings(s).map(|t| unquote(t).to_string()).collect(),
        )),
        K::Point => parse_point(s).map(ConfigValue::Point),
        K::Points | K::PointsGroups => split_vector(s)
            .map(parse_point)
            .collect::<Option<Vec<Vec<f64>>>>()
            .map(ConfigValue::Points),
    }
}

/// Sérialise une valeur au format texte Orca (inverse de `parse_orca_value`).
pub fn serialize_orca_value(value: &ConfigValue) -> String {
    match value {
        ConfigValue::Bool(b) => if *b { "1" } else { "0" }.into(),
        ConfigValue::Int(i) => i.to_string(),
        ConfigValue::Float(x) => format_float(*x),
        ConfigValue::FloatOrPercent { value, percent } => {
            if *percent {
                format!("{}%", format_float(*value))
            } else {
                format_float(*value)
            }
        }
        ConfigValue::String(s) => s.clone(),
        ConfigValue::Point(p) => serialize_point(p),
        ConfigValue::Bools(v) => join(v.iter().map(|b| if *b { "1".into() } else { "0".into() })),
        ConfigValue::Ints(v) => join(v.iter().map(|i| i.to_string())),
        ConfigValue::Floats(v) => join(v.iter().map(|x| format_float(*x))),
        ConfigValue::FloatsNullable(v) => {
            join(v.iter().map(|x| x.map_or("nil".into(), format_float)))
        }
        ConfigValue::Strings(v) => v
            .iter()
            .map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(";"),
        ConfigValue::Points(v) => join(v.iter().map(|p| serialize_point(p))),
    }
}

fn join(items: impl Iterator<Item = String>) -> String {
    items.collect::<Vec<_>>().join(",")
}

fn parse_float(s: &str) -> Option<f64> {
    s.trim().parse().ok()
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

fn parse_point(s: &str) -> Option<Vec<f64>> {
    let coords: Option<Vec<f64>> = s.trim().split('x').map(|c| c.trim().parse().ok()).collect();
    coords.filter(|c| c.len() >= 2)
}

fn serialize_point(p: &[f64]) -> String {
    p.iter()
        .map(|c| format_float(*c))
        .collect::<Vec<_>>()
        .join("x")
}

fn format_float(x: f64) -> String {
    // même règle qu'Orca : représentation compacte sans zéros superflus
    let s = format!("{x}");
    s
}

fn split_vector(s: &str) -> impl Iterator<Item = &str> {
    s.split(',').map(str::trim).filter(|t| !t.is_empty())
}

/// Les vecteurs de chaînes Orca sont séparés par `;` (avec guillemets
/// optionnels) ; les scalaires multilignes ne passent pas par ici.
fn split_strings(s: &str) -> impl Iterator<Item = &str> {
    s.split(';').map(str::trim).filter(|t| !t.is_empty())
}

fn unquote(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::get;

    fn roundtrip(key: &str, text: &str) -> String {
        let def = get(key).unwrap();
        let value = parse_orca_value(def, text).unwrap_or_else(|| panic!("parse {key}={text}"));
        serialize_orca_value(&value)
    }

    #[test]
    fn scalaires() {
        assert_eq!(roundtrip("layer_height", "0.2"), "0.2");
        assert_eq!(roundtrip("wall_loops", "3"), "3");
        assert_eq!(roundtrip("enable_support", "1"), "1");
        assert_eq!(roundtrip("enable_support", "0"), "0");
        assert_eq!(roundtrip("sparse_infill_pattern", "gyroid"), "gyroid");
    }

    #[test]
    fn float_ou_pourcentage() {
        assert_eq!(roundtrip("initial_layer_line_width", "0.5"), "0.5");
        // sparse_infill_density est un Percent : « 15% » accepté
        let def = get("sparse_infill_density").unwrap();
        assert_eq!(parse_orca_value(def, "15%"), Some(ConfigValue::Float(15.0)));
    }

    #[test]
    fn vecteurs() {
        assert_eq!(roundtrip("machine_max_speed_x", "500,200"), "500,200");
        let def = get("nozzle_diameter").unwrap();
        assert_eq!(
            parse_orca_value(def, "0.4,0.4"),
            Some(ConfigValue::Floats(vec![0.4, 0.4]))
        );
    }

    #[test]
    fn vecteur_nullable_avec_nil() {
        let def = get("filament_retraction_length").unwrap();
        assert_eq!(
            parse_orca_value(def, "0.8,nil"),
            Some(ConfigValue::FloatsNullable(vec![Some(0.8), None]))
        );
        assert_eq!(
            roundtrip("filament_retraction_length", "0.8,nil"),
            "0.8,nil"
        );
    }

    #[test]
    fn points() {
        let def = get("printable_area").unwrap();
        let v = parse_orca_value(def, "0x0,200x0,200x200,0x200").unwrap();
        assert_eq!(
            v,
            ConfigValue::Points(vec![
                vec![0.0, 0.0],
                vec![200.0, 0.0],
                vec![200.0, 200.0],
                vec![0.0, 200.0]
            ])
        );
        assert_eq!(serialize_orca_value(&v), "0x0,200x0,200x200,0x200");
    }

    #[test]
    fn chaines_multiples_avec_guillemets() {
        let def = get("filament_type").unwrap();
        assert_eq!(
            parse_orca_value(def, "\"PLA\";\"PETG\""),
            Some(ConfigValue::Strings(vec!["PLA".into(), "PETG".into()]))
        );
    }

    #[test]
    fn valeur_invalide_refusee() {
        let def = get("layer_height").unwrap();
        assert_eq!(parse_orca_value(def, "abc"), None);
    }
}
