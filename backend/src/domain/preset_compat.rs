//! Compatibilité preset ↔ imprimante, comme OrcaSlicer
//! (`Preset::is_compatible_with_printer`, `libslic3r/Preset.cpp`) :
//!
//! 1. `compatible_printers` et `compatible_printers_condition` sont résolus en
//!    remontant la chaîne `inherits` (la valeur la plus proche de la feuille
//!    gagne, même vide — comme l'application des parents au chargement Orca) ;
//! 2. liste vide + condition non vide → la condition est évaluée sur la config
//!    **résolue** de l'imprimante active (+ `printer_preset`, `num_extruders`) ;
//!    une erreur d'évaluation rend le preset compatible (FIXME Orca identique) ;
//! 3. sinon : liste vide → universel ; liste non vide → nom de l'imprimante
//!    présent, ou (imprimante utilisateur) nom de son parent système.
//!
//! L'évaluateur couvre le sous-ensemble de `PlaceholderParser` utilisé par
//! les conditions des profils : `or`/`||`, `and`/`&&`, `not`/`!`,
//! parenthèses, `== != < > <= >=`, `=~`/`!~` (regex, correspondance
//! complète comme `boost::regex_match`, `.` incluant le saut de ligne),
//! identifiants avec index `[n]`, nombres, chaînes, `true`/`false`.
//! Fonctions pures, sans E/S.

use std::collections::HashMap;

use regex::Regex;
use serde_json::{Map, Value};

/// Compatibilité effective d'un preset (chaîne d'héritage résolue).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompatSpec {
    pub compatible_printers: Vec<String>,
    pub condition: String,
}

/// Imprimante active : nom, parent (imprimante utilisateur) et config résolue.
#[derive(Debug, Clone)]
pub struct ActivePrinter {
    pub name: String,
    /// Parent système si l'imprimante est un preset utilisateur.
    pub user_parent: Option<String>,
    /// Config résolue (clés Orca → JSON), enrichie de `printer_preset` et
    /// `num_extruders` comme `is_compatible_with_printer`.
    pub config: Map<String, Value>,
}

impl ActivePrinter {
    pub fn new(name: String, user_parent: Option<String>, mut config: Map<String, Value>) -> Self {
        config.insert("printer_preset".into(), Value::String(name.clone()));
        if let Some(Value::Array(n)) = config.get("nozzle_diameter") {
            let count = n.len();
            config.insert("num_extruders".into(), Value::from(count));
        }
        Self {
            name,
            user_parent,
            config,
        }
    }
}

fn strings(v: &Value) -> Vec<String> {
    match v {
        Value::Array(a) => a
            .iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect(),
        Value::String(s) if !s.is_empty() => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn condition_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        // Certains profils stockent les scalaires en tableau d'une chaîne.
        Value::Array(a) => a.first().and_then(Value::as_str).unwrap_or("").to_string(),
        _ => String::new(),
    }
}

/// Résout la compatibilité d'une chaîne `[feuille, parent, …, racine]` de
/// valeurs brutes : pour chaque clé, la première occurrence gagne.
pub fn resolve_spec<'a>(chain: impl IntoIterator<Item = &'a Map<String, Value>>) -> CompatSpec {
    let mut printers: Option<Vec<String>> = None;
    let mut condition: Option<String> = None;
    for values in chain {
        if printers.is_none() {
            if let Some(v) = values.get("compatible_printers") {
                printers = Some(strings(v));
            }
        }
        if condition.is_none() {
            if let Some(v) = values.get("compatible_printers_condition") {
                condition = Some(condition_string(v));
            }
        }
        if printers.is_some() && condition.is_some() {
            break;
        }
    }
    CompatSpec {
        compatible_printers: printers.unwrap_or_default(),
        condition: condition.unwrap_or_default().trim().to_string(),
    }
}

/// Cache de regex compilées (une liste de presets réutilise ~50 motifs).
#[derive(Default)]
pub struct RegexCache(HashMap<String, Option<Regex>>);

impl RegexCache {
    fn get(&mut self, pattern: &str) -> Option<&Regex> {
        self.0
            .entry(pattern.to_string())
            .or_insert_with(|| Regex::new(&format!("(?s)^(?:{pattern})$")).ok())
            .as_ref()
    }
}

/// `Preset::is_compatible_with_printer` (Orca, compatibilité inter-vendeurs).
pub fn is_compatible(spec: &CompatSpec, printer: &ActivePrinter, cache: &mut RegexCache) -> bool {
    if spec.compatible_printers.is_empty() && !spec.condition.is_empty() {
        return evaluate_condition(&spec.condition, &printer.config, cache).unwrap_or(true);
    }
    spec.compatible_printers.is_empty()
        || spec.compatible_printers.contains(&printer.name)
        || printer
            .user_parent
            .as_ref()
            .is_some_and(|parent| spec.compatible_printers.iter().any(|p| p == parent))
}

// --- Évaluateur -------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Num(f64),
    Str(String),
    Regex(String),
    Op(&'static str),
    LParen,
    RParen,
    LBracket,
    RBracket,
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        let op2 = ["==", "!=", "<=", ">=", "=~", "!~", "&&", "||"]
            .into_iter()
            .find(|o| *o == two);
        if let Some(op) = op2 {
            out.push(Tok::Op(op));
            i += 2;
            continue;
        }
        match c {
            '(' => out.push(Tok::LParen),
            ')' => out.push(Tok::RParen),
            '[' => out.push(Tok::LBracket),
            ']' => out.push(Tok::RBracket),
            '<' => out.push(Tok::Op("<")),
            '>' => out.push(Tok::Op(">")),
            '!' => out.push(Tok::Op("!")),
            '"' => {
                let mut s = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                    }
                    s.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return Err("chaîne non terminée".into());
                }
                out.push(Tok::Str(s));
            }
            '/' => {
                let mut s = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '/' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        s.push('\\');
                        i += 1;
                    }
                    s.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return Err("regex non terminée".into());
                }
                out.push(Tok::Regex(s));
            }
            c if c.is_ascii_digit()
                || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())) =>
            {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == 'e')
                {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                out.push(Tok::Num(
                    text.parse()
                        .map_err(|_| format!("nombre invalide {text}"))?,
                ));
                continue;
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                out.push(match word.as_str() {
                    "and" => Tok::Op("&&"),
                    "or" => Tok::Op("||"),
                    "not" => Tok::Op("!"),
                    _ => Tok::Ident(word),
                });
                continue;
            }
            other => return Err(format!("caractère inattendu « {other} »")),
        }
        i += 1;
    }
    Ok(out)
}

#[derive(Debug, Clone)]
enum Val {
    Bool(bool),
    Num(f64),
    Str(String),
}

impl Val {
    fn truthy(&self) -> Result<bool, String> {
        match self {
            Val::Bool(b) => Ok(*b),
            Val::Num(n) => Ok(*n != 0.0),
            // Option absente (défaut Orca vide) ou chaîne « 0 »/« 1 » sérialisée.
            Val::Str(s) => Ok(!s.is_empty() && s != "0" && s != "false"),
        }
    }
    fn num(&self) -> Option<f64> {
        match self {
            Val::Num(n) => Some(*n),
            Val::Bool(b) => Some(f64::from(u8::from(*b))),
            Val::Str(s) => s.trim().parse().ok(),
        }
    }
    fn text(&self) -> String {
        match self {
            Val::Str(s) => s.clone(),
            Val::Num(n) => n.to_string(),
            Val::Bool(b) => b.to_string(),
        }
    }
}

fn json_to_val(v: &Value) -> Result<Val, String> {
    match v {
        Value::Bool(b) => Ok(Val::Bool(*b)),
        Value::Number(n) => Ok(Val::Num(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => Ok(Val::Str(s.clone())),
        Value::Null => Err("valeur nulle".into()),
        _ => Err("valeur composée".into()),
    }
}

struct Parser<'a> {
    toks: Vec<Tok>,
    pos: usize,
    config: &'a Map<String, Value>,
    cache: &'a mut RegexCache,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn eat_op(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Op(o)) if *o == op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn or(&mut self) -> Result<Val, String> {
        let mut v = self.and()?;
        while self.eat_op("||") {
            let r = self.and()?;
            v = Val::Bool(v.truthy()? || r.truthy()?);
        }
        Ok(v)
    }
    fn and(&mut self) -> Result<Val, String> {
        let mut v = self.not()?;
        while self.eat_op("&&") {
            let r = self.not()?;
            v = Val::Bool(v.truthy()? && r.truthy()?);
        }
        Ok(v)
    }
    fn not(&mut self) -> Result<Val, String> {
        if self.eat_op("!") {
            return Ok(Val::Bool(!self.not()?.truthy()?));
        }
        self.cmp()
    }
    fn cmp(&mut self) -> Result<Val, String> {
        let lhs = self.primary()?;
        let op = match self.peek() {
            Some(Tok::Op(o)) if ["==", "!=", "<", ">", "<=", ">=", "=~", "!~"].contains(o) => *o,
            _ => return Ok(lhs),
        };
        self.pos += 1;
        if op == "=~" || op == "!~" {
            let Some(Tok::Regex(pattern)) = self.peek().cloned() else {
                return Err("regex attendue après =~".into());
            };
            self.pos += 1;
            let re = self.cache.get(&pattern).ok_or("regex invalide")?;
            let m = re.is_match(&lhs.text());
            return Ok(Val::Bool(if op == "=~" { m } else { !m }));
        }
        let rhs = self.primary()?;
        let ord = match (lhs.num(), rhs.num(), &lhs, &rhs) {
            (_, _, Val::Str(a), Val::Str(b)) => a.cmp(b),
            (Some(a), Some(b), _, _) => {
                if (a - b).abs() < 1e-9 {
                    std::cmp::Ordering::Equal
                } else if a < b {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            }
            _ => lhs.text().cmp(&rhs.text()),
        };
        use std::cmp::Ordering::*;
        Ok(Val::Bool(match op {
            "==" => ord == Equal,
            "!=" => ord != Equal,
            "<" => ord == Less,
            ">" => ord == Greater,
            "<=" => ord != Greater,
            _ => ord != Less,
        }))
    }
    fn primary(&mut self) -> Result<Val, String> {
        let tok = self.peek().cloned().ok_or("expression incomplète")?;
        self.pos += 1;
        match tok {
            Tok::Num(n) => Ok(Val::Num(n)),
            Tok::Str(s) => Ok(Val::Str(s)),
            Tok::LParen => {
                let v = self.or()?;
                if self.peek() != Some(&Tok::RParen) {
                    return Err("« ) » attendue".into());
                }
                self.pos += 1;
                Ok(v)
            }
            Tok::Ident(name) if name == "true" => Ok(Val::Bool(true)),
            Tok::Ident(name) if name == "false" => Ok(Val::Bool(false)),
            Tok::Ident(name) => {
                let mut index = None;
                if self.peek() == Some(&Tok::LBracket) {
                    let Some(Tok::Num(n)) = self.toks.get(self.pos + 1).cloned() else {
                        return Err("index attendu".into());
                    };
                    if self.toks.get(self.pos + 2) != Some(&Tok::RBracket) {
                        return Err("« ] » attendue".into());
                    }
                    self.pos += 3;
                    index = Some(n as usize);
                }
                // La config complète d'Orca porte toutes les options : une clé
                // absente ici (sans défaut au registre, ex. `printer_notes`)
                // vaut la chaîne vide, comme son défaut Orca.
                let Some(value) = self.config.get(&name) else {
                    return Ok(Val::Str(String::new()));
                };
                match value {
                    Value::Array(items) => json_to_val(
                        items
                            .get(index.unwrap_or(0))
                            .ok_or_else(|| format!("index hors limites pour {name}"))?,
                    ),
                    scalar => json_to_val(scalar),
                }
            }
            other => Err(format!("jeton inattendu {other:?}")),
        }
    }
}

/// Évalue une condition `compatible_printers_condition` sur une config
/// d'imprimante. `Err` = expression hors du sous-ensemble ou invalide.
pub fn evaluate_condition(
    condition: &str,
    config: &Map<String, Value>,
    cache: &mut RegexCache,
) -> Result<bool, String> {
    let toks = lex(condition)?;
    let mut parser = Parser {
        toks,
        pos: 0,
        config,
        cache,
    };
    let v = parser.or()?;
    if parser.pos != parser.toks.len() {
        return Err("jetons en trop".into());
    }
    v.truthy()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cfg(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn evaluates_profile_conditions() {
        let mk4s = cfg(json!({
            "printer_notes": "Don't remove\nPRINTER_VENDOR_PRUSA3D\nPRINTER_MODEL_MK4S\nPG",
            "nozzle_diameter": [0.4],
            "printer_model": "MK4S",
            "single_extruder_multi_material": false
        }));
        let mut c = RegexCache::default();
        let ev = |s: &str, c: &mut RegexCache| evaluate_condition(s, &mk4s, c);
        assert_eq!(
            ev(
                "printer_notes=~/.*MK4S.*/ and nozzle_diameter[0]==0.4",
                &mut c
            ),
            Ok(true)
        );
        assert_eq!(
            ev(
                "printer_notes=~/.*MK4S.*/ and nozzle_diameter[0]==0.6",
                &mut c
            ),
            Ok(false)
        );
        assert_eq!(
            ev("printer_notes=~/.*MK4S.*/ and nozzle_diameter[0]==0.4 and printer_notes!~/.*HF_NOZZLE.*/", &mut c),
            Ok(true)
        );
        assert_eq!(
            ev(
                "printer_notes=~/.*PRINTER_MODEL_MK4S[^_a-zA-Z0-9].*/",
                &mut c
            ),
            Ok(true)
        );
        assert_eq!(
            ev(
                "printer_notes=~/.*PRINTER_MODEL_MK4[^_a-zA-Z0-9].*/",
                &mut c
            ),
            Ok(false)
        );
        assert_eq!(
            ev(
                "printer_model==\"MK4S\" && !(nozzle_diameter[0] > 0.5)",
                &mut c
            ),
            Ok(true)
        );
        assert_eq!(
            ev(
                "printer_model==\"MK3\" or single_extruder_multi_material",
                &mut c
            ),
            Ok(false)
        );
        assert_eq!(ev("not single_extruder_multi_material", &mut c), Ok(true));
        assert_eq!(
            ev("unknown_key == 1", &mut c),
            Ok(false),
            "clé absente = défaut vide"
        );
        assert_eq!(ev("unknown_key=~/.*MK4S.*/", &mut c), Ok(false));
        assert!(ev("printer_notes=~/unterminated", &mut c).is_err());
    }

    #[test]
    fn compatibility_follows_orca_rules() {
        let printer = ActivePrinter::new(
            "Prusa MK4S 0.4 nozzle (copie)".into(),
            Some("Prusa MK4S 0.4 nozzle".into()),
            cfg(json!({ "printer_notes": "PRINTER_MODEL_MK4S", "nozzle_diameter": [0.4] })),
        );
        let mut c = RegexCache::default();
        let spec = |list: &[&str], cond: &str| CompatSpec {
            compatible_printers: list.iter().map(|s| s.to_string()).collect(),
            condition: cond.into(),
        };
        assert!(is_compatible(&spec(&[], ""), &printer, &mut c), "universel");
        assert!(
            is_compatible(&spec(&["Prusa MK4S 0.4 nozzle"], ""), &printer, &mut c),
            "parent"
        );
        assert!(!is_compatible(
            &spec(&["Voron 2.4 350 0.4 nozzle"], ""),
            &printer,
            &mut c
        ));
        assert!(is_compatible(
            &spec(&[], "nozzle_diameter[0]==0.4"),
            &printer,
            &mut c
        ));
        assert!(!is_compatible(
            &spec(&[], "nozzle_diameter[0]==0.6"),
            &printer,
            &mut c
        ));
        // La liste prime sur la condition.
        assert!(!is_compatible(
            &spec(&["Autre"], "nozzle_diameter[0]==0.4"),
            &printer,
            &mut c
        ));
        // Erreur d'évaluation → compatible (comportement Orca).
        assert!(is_compatible(
            &spec(&[], "nozzle_diameter[0] =="),
            &printer,
            &mut c
        ));
        // Clé absente de la config (printer_notes d'une Voron) → défaut vide.
        let voron = ActivePrinter::new(
            "Voron".into(),
            None,
            cfg(json!({ "nozzle_diameter": [0.4] })),
        );
        assert!(!is_compatible(
            &spec(&[], "printer_notes=~/.*MK4S.*/"),
            &voron,
            &mut c
        ));
        assert_eq!(printer.config["num_extruders"], json!(1));
    }

    #[test]
    fn spec_resolution_nearest_wins() {
        let leaf = cfg(json!({ "layer_height": "0.2" }));
        let mid = cfg(
            json!({ "compatible_printers": [], "compatible_printers_condition": "nozzle_diameter[0]==0.4" }),
        );
        let root =
            cfg(json!({ "compatible_printers": ["X"], "compatible_printers_condition": "" }));
        let s = resolve_spec([&leaf, &mid, &root]);
        assert_eq!(
            s,
            CompatSpec {
                compatible_printers: vec![],
                condition: "nozzle_diameter[0]==0.4".into()
            }
        );
        let s = resolve_spec([&leaf, &root]);
        assert_eq!(s.compatible_printers, vec!["X".to_string()]);
        let arr = cfg(json!({ "compatible_printers_condition": ["a == 1"] }));
        assert_eq!(resolve_spec([&arr]).condition, "a == 1");
    }
}
