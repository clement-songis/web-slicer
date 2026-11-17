//! Conversion entre la sérialisation texte d'Orca (`opt_serialize`) et
//! `ConfigValue`, pilotée par le registre. Utilisée par l'échange 3MF (T014)
//! et la résolution de presets (T037).

use crate::api::ConfigValue;

use super::{ParamDef, ParamKind};

/// Parse une valeur au format texte Orca selon le type du registre.
/// `None` si la valeur est invalide pour ce type.
pub fn parse_orca_value(def: &ParamDef, s: &str) -> Option<ConfigValue> {
    use ParamKind as K;
    // Les chaînes Orca (notamment les G-code personnalisés) sont sensibles aux
    // espaces et retours à la ligne. Seules les valeurs numériques/vectorielles
    // sont normalisées par trim.
    let s = match def.kind {
        K::String | K::Enum | K::Strings | K::Enums => s,
        _ => s.trim(),
    };
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
        // `ConfigOptionBoolsTempl<NULLABLE>::deserialize` : éléments séparés
        // par `,`, `nil` admis seulement si nullable (sinon erreur). Comme pour
        // les flottants, la variante nullable n'est produite qu'en présence
        // d'un `nil` : longueur et indices multi-filament sont conservés.
        K::Bools => {
            if def.nullable && split_vector(s).any(|t| t == "nil") {
                split_vector(s)
                    .map(|t| {
                        if t == "nil" {
                            Some(None)
                        } else {
                            parse_bool(t).map(Some)
                        }
                    })
                    .collect::<Option<Vec<Option<bool>>>>()
                    .map(ConfigValue::BoolsNullable)
            } else {
                split_vector(s)
                    .map(parse_bool)
                    .collect::<Option<Vec<bool>>>()
                    .map(ConfigValue::Bools)
            }
        }
        K::Enums => parse_enums(def, s).map(ConfigValue::Strings),
        K::Strings => {
            if def.nullable && s.trim() == "nil" {
                Some(ConfigValue::Strings(Vec::new()))
            } else {
                Some(ConfigValue::Strings(
                    split_strings(s)
                        .into_iter()
                        .map(|t| unquote(t).to_string())
                        .collect(),
                ))
            }
        }
        K::Point => parse_point(s).map(ConfigValue::Point),
        // Groupes de points (extruder_printable_area : une zone par buse) :
        // sérialisation Orca « g1#g2 », gardés en chaînes « 0x0,325x0,… ».
        K::PointsGroups => Some(ConfigValue::Strings(
            s.split('#')
                .map(str::trim)
                .filter(|g| !g.is_empty())
                .map(str::to_string)
                .collect(),
        )),
        K::Points => split_vector(s)
            .map(parse_point)
            .collect::<Option<Vec<Vec<f64>>>>()
            .map(ConfigValue::Points),
    }
}

/// `ConfigOptionEnumsGenericTempl<NULLABLE>::deserialize` : éléments séparés
/// par `,` et trimés, `nil` = élément nil (seulement si nullable). Le jeton
/// `nil` est conservé tel quel dans le vecteur : aucune valeur d'enum Orca ne
/// peut s'appeler `nil` (mot réservé par le désérialiseur), la représentation
/// est donc non ambiguë et préserve longueur et indices multi-filament.
///
/// Accepte aussi la forme interne `"A";"B"` (tableaux JSON de presets/snapshots
/// joints par `;` avec guillemets) : un segment entre guillemets est un
/// élément unique, un segment nu est re-découpé sur `,`.
fn parse_enums(def: &ParamDef, s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for part in split_strings(s).into_iter().filter(|p| !p.is_empty()) {
        let quoted = part.len() >= 2 && part.starts_with('"') && part.ends_with('"');
        let items: Vec<String> = if quoted {
            vec![unquote(part).trim().to_string()]
        } else {
            part.split(',').map(|t| t.trim().to_string()).collect()
        };
        for item in items {
            if item == "nil" && !def.nullable {
                return None; // Orca : « Deserializing nil into a non-nullable object »
            }
            out.push(item);
        }
    }
    Some(out)
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
        ConfigValue::BoolsNullable(v) => join(v.iter().map(|b| match b {
            Some(true) => "1".into(),
            Some(false) => "0".into(),
            None => "nil".into(),
        })),
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

/// Sérialise en tenant compte du type du registre. `ConfigValue::Float` ne
/// porte pas la nature « pourcentage » ; sans le type, le round-trip perd le
/// suffixe `%` (« 85% » → « 85 »). On le réimpose aux clés `Percent`/`Percents`
/// pour rester **fidèle à la forme canonique d'OrcaSlicer** : un 3MF exporté
/// (FR-044) doit sérialiser « 85% », pas « 85 ». libslic3r relit un pourcentage
/// nu à l'identique (d'où aucun écart de tranchage mesuré, cf. `gcode_parity`),
/// mais la fidélité évite toute divergence latente et garde l'interop 3MF valide.
pub fn serialize_orca_value_for(def: &ParamDef, value: &ConfigValue) -> String {
    use ParamKind as K;
    if def.kind == K::Enums {
        return match value {
            // Héritage : l'ancien parseur rendait `nil` en `Strings([])` ; un
            // vecteur vide déjà persisté repart donc en `nil` (un élément nil)
            // plutôt qu'en chaîne vide (vecteur vide, invalide pour Orca).
            ConfigValue::Strings(values) if values.is_empty() && def.nullable => "nil".into(),
            ConfigValue::Strings(values) => values.join(","),
            other => serialize_orca_value(other),
        };
    }
    match (def.kind, value) {
        (K::PointsGroups, ConfigValue::Strings(groups)) => return groups.join("#"),
        // ConfigOptionPoint::serialize d'Orca : « x,y » (ex. best_object_pos).
        (K::Point, ConfigValue::Point(p)) => {
            return p
                .iter()
                .map(|c| format_float(*c))
                .collect::<Vec<_>>()
                .join(",")
        }
        _ => {}
    }
    let raw = serialize_orca_value(value);
    match def.kind {
        K::Percent => ensure_percent(&raw),
        K::Percents => raw
            .split(',')
            .map(ensure_percent)
            .collect::<Vec<_>>()
            .join(","),
        _ => raw,
    }
}

/// Ajoute `%` à une valeur numérique qui ne l'a pas (laisse `nil`/vide tels quels).
fn ensure_percent(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() || t == "nil" || t.ends_with('%') {
        t.to_string()
    } else {
        format!("{t}%")
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
    // ConfigOptionPoint d'Orca : « x,y » (sérialisation) ou « xXy » (presets).
    let sep = if s.contains('x') { 'x' } else { ',' };
    let coords: Option<Vec<f64>> = s.trim().split(sep).map(|c| c.trim().parse().ok()).collect();
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
/// optionnels) ; les scalars multilignes ne passent pas par ici.
fn split_strings(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, ch) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            ';' if !quoted => {
                out.push(s[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    // Comme unescape_strings_cstyle d'Orca : les entrées vides sont conservées
    // (« ;;; » = 4 chaînes vides, ex. compatible_machine_expression_group d'un
    // projet à 2 filaments) ; seule une valeur entièrement vide donne [].
    if s.trim().is_empty() {
        return Vec::new();
    }
    out.push(s[start..].trim());
    out
}

fn unquote(s: &str) -> String {
    let inner = s
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(s);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
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
    fn scalars() {
        assert_eq!(roundtrip("layer_height", "0.2"), "0.2");
        assert_eq!(roundtrip("wall_loops", "3"), "3");
        assert_eq!(roundtrip("enable_support", "1"), "1");
        assert_eq!(roundtrip("enable_support", "0"), "0");
        assert_eq!(roundtrip("sparse_infill_pattern", "gyroid"), "gyroid");
    }

    #[test]
    fn float_or_percent() {
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
    fn multiple_strings_with_quotes() {
        let def = get("filament_type").unwrap();
        assert_eq!(
            parse_orca_value(def, "\"PLA\";\"PETG\""),
            Some(ConfigValue::Strings(vec!["PLA".into(), "PETG".into()]))
        );
    }

    #[test]
    fn preserves_custom_gcode_comments_and_newlines() {
        let def = get("filament_start_gcode").unwrap();
        assert_eq!(
            parse_orca_value(def, "\"; Filament gcode\\n\""),
            Some(ConfigValue::Strings(vec!["; Filament gcode\n".into()]))
        );
    }

    #[test]
    fn nullable_enum_nil_keeps_length_and_indices() {
        let def = get("filament_z_hop_types").unwrap();
        // Un seul filament nil : un élément, pas un vecteur vide.
        assert_eq!(
            parse_orca_value(def, "nil"),
            Some(ConfigValue::Strings(vec!["nil".into()]))
        );
        // Forme Orca native (`opt_serialize`) : séparateur `,`, trim.
        let v = parse_orca_value(def, "Normal Lift, nil,Spiral Lift").unwrap();
        assert_eq!(
            v,
            ConfigValue::Strings(vec![
                "Normal Lift".into(),
                "nil".into(),
                "Spiral Lift".into()
            ])
        );
        assert_eq!(
            serialize_orca_value_for(def, &v),
            "Normal Lift,nil,Spiral Lift"
        );
        // Forme interne des tableaux JSON joints (`"A";"nil"`).
        assert_eq!(
            parse_orca_value(def, "\"nil\";\"Slope Lift\""),
            Some(ConfigValue::Strings(vec![
                "nil".into(),
                "Slope Lift".into()
            ]))
        );
        let enforce = get("filament_retract_lift_enforce").unwrap();
        let v = parse_orca_value(enforce, "nil,nil").unwrap();
        assert_eq!(v, ConfigValue::Strings(vec!["nil".into(), "nil".into()]));
        assert_eq!(serialize_orca_value_for(enforce, &v), "nil,nil");
    }

    #[test]
    fn non_nullable_enums_reject_nil_and_split_on_commas() {
        let def = get("extruder_type").unwrap();
        assert!(!def.nullable);
        assert_eq!(parse_orca_value(def, "nil"), None);
        assert_eq!(
            parse_orca_value(def, "Direct Drive,Bowden"),
            Some(ConfigValue::Strings(vec![
                "Direct Drive".into(),
                "Bowden".into()
            ]))
        );
    }

    #[test]
    fn nullable_bools_with_nil() {
        let def = get("filament_wipe").unwrap();
        let v = parse_orca_value(def, "1, nil,0").unwrap();
        assert_eq!(
            v,
            ConfigValue::BoolsNullable(vec![Some(true), None, Some(false)])
        );
        assert_eq!(serialize_orca_value(&v), "1,nil,0");
        assert_eq!(serialize_orca_value_for(def, &v), "1,nil,0");
        assert_eq!(
            roundtrip("filament_retract_when_changing_layer", "nil"),
            "nil"
        );
        // Sans nil, la forme historique `Bools` est conservée.
        assert_eq!(
            parse_orca_value(def, "1,0"),
            Some(ConfigValue::Bools(vec![true, false]))
        );
        // Forme snapshot backend (`true`/`false`/`nil`).
        assert_eq!(
            parse_orca_value(
                get("filament_long_retractions_when_cut").unwrap(),
                "true,nil"
            ),
            Some(ConfigValue::BoolsNullable(vec![Some(true), None]))
        );
    }

    #[test]
    fn non_nullable_bools_reject_nil() {
        let def = get("filament_soluble").unwrap();
        assert!(!def.nullable);
        assert_eq!(parse_orca_value(def, "1,nil"), None);
        assert_eq!(parse_orca_value(def, "1,x"), None);
    }

    #[test]
    fn invalid_value_rejected() {
        let def = get("layer_height").unwrap();
        assert_eq!(parse_orca_value(def, "abc"), None);
    }

    #[test]
    fn serialize_for_key_reimpose_le_pourcent() {
        // Un Percent lu depuis Orca (« 15% ») repart « 15% », pas « 15 ».
        let def = get("sparse_infill_density").unwrap();
        let v = parse_orca_value(def, "15%").unwrap();
        assert_eq!(serialize_orca_value(&v), "15"); // forme nue (perte du %)
        assert_eq!(serialize_orca_value_for(def, &v), "15%"); // forme canonique

        // Un Float ordinaire n'est pas affublé de %.
        let lh = get("layer_height").unwrap();
        let hv = parse_orca_value(lh, "0.2").unwrap();
        assert_eq!(serialize_orca_value_for(lh, &hv), "0.2");
    }

    #[test]
    fn enums_are_serialized_without_string_quotes() {
        let def = get("z_hop_types").unwrap();
        let value = ConfigValue::Strings(vec!["Normal Lift".into()]);
        assert_eq!(serialize_orca_value_for(def, &value), "Normal Lift");
        let nullable = get("filament_z_hop_types").unwrap();
        assert_eq!(
            serialize_orca_value_for(nullable, &ConfigValue::Strings(Vec::new())),
            "nil"
        );
    }
}
