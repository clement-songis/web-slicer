//! Nom de fichier d'un G-code exporté, convention OrcaSlicer.
//!
//! Orca nomme l'export avec le gabarit `filename_format` du process
//! (`PrintConfig.cpp`, défaut `{input_filename_base}_{filament_type[initial_tool]}_{print_time}.gcode`)
//! évalué par `PrintBase::output_filename` : `input_filename_base` = nom du
//! projet (`Plater::export_gcode` → `output_filepath_for_project`),
//! `print_time` = `short_time(estimated_normal_print_time)`
//! (`PrintStatistics::config`, `Utils.hpp`), le reste vient de la
//! configuration. Le chemin est ensuite replié en ASCII (`fold_utf8_to_ascii`).
//!
//! Ici tout est relu dans le G-code stocké : bloc `; CONFIG_BLOCK_START` (valeurs
//! du preset, dont `filename_format`) et en-tête `; estimated printing time`.
//! Sous-ensemble du PlaceholderParser : variables `{nom}`, `{nom[i]}`,
//! `{nom[variable]}` et syntaxe historique `[nom]` ; toute expression non
//! supportée (conditions, calculs…) fait retomber sur le gabarit par défaut.

use std::collections::HashMap;

/// Valeur par défaut de `filename_format` (OrcaSlicer `PrintConfig.cpp`).
pub const DEFAULT_FILENAME_FORMAT: &str =
    "{input_filename_base}_{filament_type[initial_tool]}_{print_time}.gcode";

/// Paires `clé = valeur` du bloc de configuration en fin de G-code.
pub fn config_block(gcode: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut inside = false;
    for line in gcode.lines() {
        let Some(comment) = line.trim_start().strip_prefix(';') else {
            continue;
        };
        let comment = comment.trim();
        if comment == "CONFIG_BLOCK_START" {
            inside = true;
            continue;
        }
        if comment == "CONFIG_BLOCK_END" {
            break;
        }
        if inside {
            if let Some((k, v)) = comment.split_once(" = ") {
                out.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    out
}

/// Port de `Slic3r::short_time` : `"1h 2m 40s"` → `"1h3m"`, `"8m 52s"` → `"8m52s"`.
pub fn short_time(time: &str) -> String {
    let (mut days, mut hours, mut minutes, mut seconds) = (0u64, 0u64, 0u64, 0u64);
    let mut fractional = 0.0f64;
    for token in time.split_whitespace() {
        let (num, unit) = token.split_at(token.len().saturating_sub(1));
        match unit {
            "d" => days = num.parse().unwrap_or(0),
            "h" => hours = num.parse().unwrap_or(0),
            "m" => minutes = num.parse().unwrap_or(0),
            "s" => {
                let f: f64 = num.parse().unwrap_or(0.0);
                fractional = f;
                seconds = f as u64;
            }
            _ => {}
        }
    }
    // Arrondi à la minute dès qu'il y a des heures ou des jours.
    if days + hours > 0 && seconds >= 30 {
        minutes += 1;
        if minutes == 60 {
            minutes = 0;
            hours += 1;
            if hours == 24 {
                hours = 0;
                days += 1;
            }
        }
    }
    if days > 0 {
        format!("{days}d{hours}h{minutes}m")
    } else if hours > 0 {
        format!("{hours}h{minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m{seconds}s")
    } else if seconds >= 1 {
        format!("{seconds}s")
    } else if fractional > 0.0 && fractional < 1.0 {
        "<1s".to_string()
    } else {
        "0s".to_string()
    }
}

/// Premier outil utilisé (`initial_tool`) : première commande `T<n>`, sinon 0.
fn initial_tool(gcode: &str) -> usize {
    gcode
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with(';'))
        .find_map(|l| {
            let rest = l.strip_prefix('T')?;
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            let tail = &rest[digits.len()..];
            (!digits.is_empty() && (tail.is_empty() || tail.starts_with([' ', ';'])))
                .then(|| digits.parse().ok())
                .flatten()
        })
        .unwrap_or(0)
}

/// Éléments d'un vecteur sérialisé Orca (`a;b` pour les chaînes, `a,b` pour les nombres).
fn vector_items(raw: &str) -> Vec<String> {
    let sep = if raw.contains(';') { ';' } else { ',' };
    raw.split(sep)
        .map(|s| s.trim().trim_matches('"').to_string())
        .collect()
}

fn is_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Évalue `nom` ou `nom[index]` ; `None` = expression non supportée.
fn eval(expr: &str, vars: &HashMap<String, String>) -> Option<String> {
    let expr = expr.trim();
    if let Some((name, rest)) = expr.split_once('[') {
        let index = rest.strip_suffix(']')?.trim();
        if !is_ident(name.trim()) {
            return None;
        }
        let i: usize = match index.parse() {
            Ok(i) => i,
            Err(_) if is_ident(index) => vars.get(index)?.parse().ok()?,
            Err(_) => return None,
        };
        let items = vector_items(vars.get(name.trim())?);
        return items.get(i).or_else(|| items.first()).cloned();
    }
    if !is_ident(expr) {
        return None;
    }
    let raw = vars.get(expr)?;
    Some(raw.trim_matches('"').to_string())
}

/// Applique le gabarit ; `None` si une expression n'est pas supportée.
fn process(format: &str, vars: &HashMap<String, String>) -> Option<String> {
    let mut out = String::new();
    let mut rest = format;
    while let Some(pos) = rest.find(['{', '[']) {
        out.push_str(&rest[..pos]);
        let open = rest.as_bytes()[pos];
        let close = if open == b'{' { '}' } else { ']' };
        let after = &rest[pos + 1..];
        // `{…}` : peut contenir un index `[…]` ; on prend la première `}`.
        let end = after.find(close)?;
        let inner = &after[..end];
        if open == b'[' && !(is_ident(inner) && vars.contains_key(inner)) {
            // Crochet littéral (pas une variable historique connue).
            out.push('[');
            rest = after;
            continue;
        }
        out.push_str(&eval(inner, vars)?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// Replie en ASCII sûr pour un nom de fichier (accents latins, séparateurs).
fn sanitize(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let folded: &str = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "A",
            'ç' => "c",
            'Ç' => "C",
            'è' | 'é' | 'ê' | 'ë' => "e",
            'È' | 'É' | 'Ê' | 'Ë' => "E",
            'ì' | 'í' | 'î' | 'ï' => "i",
            'Ì' | 'Í' | 'Î' | 'Ï' => "I",
            'ñ' => "n",
            'Ñ' => "N",
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => "o",
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => "O",
            'ù' | 'ú' | 'û' | 'ü' => "u",
            'Ù' | 'Ú' | 'Û' | 'Ü' => "U",
            'ý' | 'ÿ' => "y",
            'æ' => "ae",
            'Æ' => "AE",
            'œ' => "oe",
            'Œ' => "OE",
            'ß' => "ss",
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => "_",
            c if c.is_control() => "",
            c if c.is_ascii() => {
                out.push(c);
                continue;
            }
            _ => "_",
        };
        out.push_str(folded);
    }
    out.trim().to_string()
}

/// Base Orca d'un projet : nom sans extension de modèle, « Untitled » si vide.
fn project_base(filename_base: &str) -> String {
    let b = filename_base.trim();
    let b = b
        .strip_suffix(".3mf")
        .or_else(|| b.strip_suffix(".stl"))
        .unwrap_or(b);
    if b.is_empty() { "Untitled" } else { b }.to_string()
}

/// Nom de téléchargement à partir du nom calculé par libslic3r au tranchage
/// (`Print::output_filename` avec la base = jeton) : jeton remplacé par la base
/// du projet puis repli ASCII (`fold_utf8_to_ascii` côté Orca).
pub fn from_engine(template: &str, filename_base: &str) -> String {
    let base = project_base(filename_base);
    let name = sanitize(&template.replace(engine::api::OUTPUT_FILENAME_BASE_TOKEN, &base));
    if name.is_empty() || name.trim_start_matches('.').is_empty() {
        format!("{}.gcode", sanitize(&base))
    } else {
        name
    }
}

/// Nom Orca d'un G-code stocké (téléchargement, envoi à l'imprimante comme
/// `Plater::send_gcode_legacy`) : nom calculé par libslic3r au tranchage
/// (`stats.output_filename`), sinon relecture du bloc de configuration.
pub fn for_gcode(stats: &serde_json::Value, gcode: &[u8], project_name: &str) -> String {
    match stats.get("output_filename").and_then(|v| v.as_str()) {
        Some(template) => from_engine(template, project_name),
        None => output_filename(&String::from_utf8_lossy(gcode), project_name),
    }
}

/// Noms uniques dans un même envoi : Orca envoie plateau par plateau sous le
/// même gabarit (le second écrase le premier) ; ici un doublon reçoit un
/// suffixe `_<n>` avant l'extension pour ne rien écraser côté imprimante.
pub fn dedupe(names: &mut [String]) {
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for name in names.iter_mut() {
        let count = seen.entry(name.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            let (stem, ext) = match name.rsplit_once('.') {
                Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
                _ => (name.clone(), String::new()),
            };
            *name = format!("{stem}_{count}{ext}");
        }
    }
}

/// Refus d'un nom de fichier d'envoi choisi par l'utilisateur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadNameError {
    /// Nom vide (ou seulement l'extension).
    Empty,
    /// Caractères interdits (séparateurs de chemin, réservés FAT/Windows,
    /// caractères de contrôle) — liste des caractères fautifs.
    Chars(String),
    /// Nom commençant par « . » (fichier caché, ignoré par Moonraker/OctoPrint).
    Hidden,
    /// Plus de 200 octets (limite prudente des systèmes de fichiers des hôtes).
    TooLong,
}

impl UploadNameError {
    /// Code machine (`details.reason`).
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Chars(_) => "chars",
            Self::Hidden => "hidden",
            Self::TooLong => "too_long",
        }
    }

    /// Message lisible (FR-032).
    pub fn message(&self) -> String {
        match self {
            Self::Empty => "Nom de fichier requis.".into(),
            Self::Chars(bad) => format!(
                "Nom de fichier invalide : caractères interdits {bad} (/ \\ : * ? \" < > | et caractères de contrôle)."
            ),
            Self::Hidden => "Nom de fichier invalide : il ne doit pas commencer par « . ».".into(),
            Self::TooLong => "Nom de fichier trop long (200 octets au plus).".into(),
        }
    }
}

/// Longueur maximale (octets) d'un nom d'envoi.
pub const MAX_UPLOAD_NAME_BYTES: usize = 200;

/// Nom d'envoi choisi dans la boîte d'envoi (Orca `PrintHostSendDialog` :
/// nom modifiable, l'extension attendue est celle du nom proposé — `.gcode` ou
/// `.gcode.3mf`). Orca demande confirmation si l'extension manque ; ici elle
/// est **ajoutée** (`extension`, comparaison sans casse) pour que l'hôte
/// reconnaisse toujours le fichier. Contrairement à Orca, les sous-dossiers
/// (« / ») sont refusés : Moonraker les ignore dans Orca 2.4.1 (Moonraker.cpp
/// n'envoie que `upload_path.filename()`) et un nom de fichier seul reste
/// valable pour tous les hôtes. Caractères réservés refusés (mêmes que le
/// repli `sanitize` des noms automatiques) ; les accents sont gardés tels quels.
pub fn upload_name(raw: &str, extension: &str) -> Result<String, UploadNameError> {
    let name = raw.trim();
    let mut bad: Vec<char> = name
        .chars()
        .filter(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        .collect();
    bad.dedup();
    if !bad.is_empty() {
        let shown: Vec<String> = bad
            .iter()
            .map(|c| {
                if c.is_control() {
                    format!("U+{:04X}", *c as u32)
                } else {
                    format!("« {c} »")
                }
            })
            .collect();
        return Err(UploadNameError::Chars(shown.join(" ")));
    }
    let lower = name.to_lowercase();
    let ext_lower = extension.to_lowercase();
    let full = if lower.ends_with(&ext_lower) {
        name.to_string()
    } else if ext_lower == ".gcode.3mf" && lower.ends_with(".gcode") {
        format!("{name}.3mf")
    } else {
        format!("{name}{extension}")
    };
    let stem_len = full.len() - extension.len();
    if full[..stem_len].trim().is_empty() {
        return Err(UploadNameError::Empty);
    }
    if full.starts_with('.') {
        return Err(UploadNameError::Hidden);
    }
    if full.len() > MAX_UPLOAD_NAME_BYTES {
        return Err(UploadNameError::TooLong);
    }
    Ok(full)
}

/// Extension d'un nom d'envoi proposé (`.gcode.3mf` prioritaire sur `.gcode`),
/// comme `m_valid_suffix` d'Orca (dernier point du nom proposé, ici étendu au
/// double suffixe du G-code 3MF).
pub fn upload_extension(default_name: &str) -> &'static str {
    if default_name.to_lowercase().ends_with(".gcode.3mf") {
        ".gcode.3mf"
    } else {
        ".gcode"
    }
}

/// Nom de téléchargement du G-code `gcode` pour un projet nommé `filename_base`.
/// Repli pour les G-code sans nom calculé par le moteur (antérieurs, ou
/// ouverts depuis un fichier) : sous-ensemble du PlaceholderParser.
pub fn output_filename(gcode: &str, filename_base: &str) -> String {
    let mut vars = config_block(gcode);
    let base = project_base(filename_base);
    let estimated = gcode
        .lines()
        .find_map(|l| {
            l.trim_start()
                .strip_prefix(';')?
                .trim()
                .strip_prefix("estimated printing time (normal mode) = ")
                .map(|v| v.trim().to_string())
        })
        .unwrap_or_default();
    let silent = gcode
        .lines()
        .find_map(|l| {
            l.trim_start()
                .strip_prefix(';')?
                .trim()
                .strip_prefix("estimated printing time (silent mode) = ")
                .map(|v| v.trim().to_string())
        })
        .unwrap_or_else(|| estimated.clone());
    let tool = initial_tool(gcode).to_string();
    vars.insert("input_filename_base".into(), base.clone());
    vars.insert("input_filename".into(), format!("{base}.gcode"));
    vars.insert("print_time".into(), short_time(&estimated));
    vars.insert("normal_print_time".into(), short_time(&estimated));
    vars.insert("silent_print_time".into(), short_time(&silent));
    vars.insert("initial_tool".into(), tool.clone());
    vars.insert("initial_extruder".into(), tool);

    let format = vars
        .get("filename_format")
        .map(|f| f.trim_matches('"').to_string())
        .filter(|f| !f.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_FILENAME_FORMAT.to_string());
    let name = process(&format, &vars)
        .or_else(|| process(DEFAULT_FILENAME_FORMAT, &vars))
        .unwrap_or_else(|| format!("{base}.gcode"));
    let mut name = sanitize(&name);
    if name.is_empty() || name.trim_start_matches('.').is_empty() {
        name = format!("{}.gcode", sanitize(&base));
    }
    // Orca n'ajoute l'extension que si le gabarit n'en a aucune.
    let has_ext = name
        .rsplit_once('.')
        .is_some_and(|(stem, ext)| !stem.is_empty() && !ext.is_empty());
    if !has_ext {
        name.push_str(".gcode");
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_names_in_one_upload_get_a_suffix() {
        let mut names = vec![
            "Cube_PLA_1m.gcode".to_string(),
            "Cube_PLA_1m.gcode".to_string(),
            "Cube_PLA_2m.gcode".to_string(),
        ];
        dedupe(&mut names);
        assert_eq!(
            names,
            [
                "Cube_PLA_1m.gcode",
                "Cube_PLA_1m_2.gcode",
                "Cube_PLA_2m.gcode"
            ]
        );
    }

    #[test]
    fn engine_name_gets_the_project_base_and_ascii_folding() {
        let t = format!(
            "{}_PLA_8m52s.gcode",
            engine::api::OUTPUT_FILENAME_BASE_TOKEN
        );
        assert_eq!(
            from_engine(&t, "Pièce été.3mf"),
            "Piece ete_PLA_8m52s.gcode"
        );
        assert_eq!(from_engine(&t, "  "), "Untitled_PLA_8m52s.gcode");
        // Gabarit évalué vide par libslic3r : base du projet.
        assert_eq!(from_engine("", "Cube"), "Cube.gcode");
    }

    fn sample(format_line: Option<&str>) -> String {
        let mut s = String::from(
            "; generated by OrcaSlicer\n; estimated printing time (normal mode) = 8m 52s\nG28\nT0\nG1 X1 E1\n; CONFIG_BLOCK_START\n; filament_type = PLA;PETG\n; layer_height = 0.2\n",
        );
        if let Some(f) = format_line {
            s.push_str(&format!("; filename_format = {f}\n"));
        }
        s.push_str("; CONFIG_BLOCK_END\n");
        s
    }

    #[test]
    fn short_time_matches_orca() {
        assert_eq!(short_time("8m 52s"), "8m52s");
        assert_eq!(short_time("1h 2m 40s"), "1h3m");
        assert_eq!(short_time("1h 59m 30s"), "2h0m");
        assert_eq!(short_time("2d 3h 4m 5s"), "2d3h4m");
        assert_eq!(short_time("45s"), "45s");
        assert_eq!(short_time("0.5s"), "<1s");
        assert_eq!(short_time(""), "0s");
    }

    #[test]
    fn default_orca_format() {
        let g = sample(Some(DEFAULT_FILENAME_FORMAT));
        assert_eq!(output_filename(&g, "cube20"), "cube20_PLA_8m52s.gcode");
        // Bloc sans filename_format : même défaut qu'Orca.
        assert_eq!(
            output_filename(&sample(None), "cube20"),
            "cube20_PLA_8m52s.gcode"
        );
    }

    #[test]
    fn preset_format_and_indexes() {
        let g = sample(Some(
            "{input_filename_base}-{filament_type[1]}-{layer_height}mm.gcode",
        ));
        assert_eq!(output_filename(&g, "Pièce"), "Piece-PETG-0.2mm.gcode");
        let g = sample(Some("[input_filename_base]_{print_time}.gcode"));
        assert_eq!(output_filename(&g, "a/b"), "a_b_8m52s.gcode");
    }

    #[test]
    fn unsupported_expression_falls_back_to_default() {
        let g = sample(Some("{if layer_height > 0.1}fine{endif}.gcode"));
        assert_eq!(output_filename(&g, "cube20"), "cube20_PLA_8m52s.gcode");
    }

    #[test]
    fn initial_tool_selects_filament() {
        let g = sample(None).replace("T0\n", "T1\n");
        assert_eq!(output_filename(&g, "x"), "x_PETG_8m52s.gcode");
    }

    #[test]
    fn upload_name_forces_the_extension_and_keeps_accents() {
        assert_eq!(upload_name(" pièce ", ".gcode").unwrap(), "pièce.gcode");
        assert_eq!(upload_name("cube.GCODE", ".gcode").unwrap(), "cube.GCODE");
        assert_eq!(upload_name("cube.gco", ".gcode").unwrap(), "cube.gco.gcode");
        assert_eq!(
            upload_name("cube.gcode", ".gcode.3mf").unwrap(),
            "cube.gcode.3mf"
        );
        assert_eq!(upload_name("cube", ".gcode.3mf").unwrap(), "cube.gcode.3mf");
        assert_eq!(
            upload_name("v1.2 test", ".gcode").unwrap(),
            "v1.2 test.gcode"
        );
    }

    #[test]
    fn upload_name_refuses_unsafe_names() {
        assert_eq!(upload_name("  ", ".gcode"), Err(UploadNameError::Empty));
        assert_eq!(upload_name(".gcode", ".gcode"), Err(UploadNameError::Empty));
        assert_eq!(
            upload_name(".hidden", ".gcode"),
            Err(UploadNameError::Hidden)
        );
        assert_eq!(upload_name("a/b", ".gcode").unwrap_err().reason(), "chars");
        assert_eq!(
            upload_name("..\\x", ".gcode").unwrap_err().reason(),
            "chars"
        );
        assert_eq!(upload_name("a\nb", ".gcode").unwrap_err().reason(), "chars");
        let err = upload_name("a<b>?", ".gcode").unwrap_err();
        assert!(
            err.message().contains("« < »") && err.message().contains("« ? »"),
            "{}",
            err.message()
        );
        assert_eq!(
            upload_name(&"x".repeat(200), ".gcode"),
            Err(UploadNameError::TooLong)
        );
        assert!(upload_name(&"x".repeat(194), ".gcode").is_ok());
    }

    #[test]
    fn upload_extension_follows_the_proposed_name() {
        assert_eq!(upload_extension("a_PLA_1h.gcode"), ".gcode");
        assert_eq!(upload_extension("a_PLA_1h.gcode.3mf"), ".gcode.3mf");
    }
}
