//! Signature de tranchage d'un plateau (lot 42, audit 41 V1).
//!
//! Orca ne rouvre jamais un résultat de tranchage périmé : à l'ouverture d'un
//! projet, le processus d'arrière-plan est réinitialisé
//! (`Plater::priv::load_files`) ; seul un `.gcode.3mf` tranché se rouvre, figé.
//! Ici le dernier G-code d'un projet est conservé côté serveur : l'interface
//! ne doit le restaurer que s'il correspond **encore** au projet. La
//! signature rend cette comparaison possible sans stockage local du
//! navigateur (autre navigateur, stockage effacé…).
//!
//! # Calcul (version `v1`)
//!
//! `signature = "v1:" + hex(SHA-256(JSON canonique de E))` où `E` est l'objet :
//!
//! | clé | contenu |
//! |---|---|
//! | `plate` | index du plateau (0-based) |
//! | `settings` | config **effective** tranchée : presets actifs résolus (héritage Orca), surcharges `overrides` des presets (lot 40) ou brouillon de l'éditeur, réglages projet (`scene.projectConfig`, `calibrationOverrides`), réglages du plateau (`with_plate_settings`) — exactement le `resolved_settings` figé dans le job |
//! | `objects` | objets de la scène appartenant au plateau, dans l'ordre de la scène, JSON complet (modèle, transformation, volumes, réglages par objet, peinture, profil de couches…) ; un objet absent de tous les plateaux compte pour le plateau 0, comme au tranchage |
//! | `customGcodes` | changements de couleur / pauses / G-code perso du plateau (`plates[n].customGcodes`), `null` sinon |
//! | `calibration` | paramètres du mode calibration (`scene.calibration`), `null` sinon |
//!
//! JSON canonique : clés d'objets triées (ordre des octets UTF-8), aucun
//! blanc, chaînes et nombres au format de serde_json (nombres relus tels
//! qu'enregistrés : `10` et `10.0` diffèrent). Ne participent pas : le reste
//! de la scène (caméra, plateau actif, nom, autres plateaux), l'option
//! « Remove mixed temperature restriction » (elle autorise le tranchage sans
//! changer le G-code).
//!
//! Conséquences : modifier un réglage, une surcharge, un preset utilisateur
//! utilisé, un objet du plateau (position, échelle, peinture…) ou ses
//! changements de couleur change la signature ; déplacer un objet d'un autre
//! plateau ne change pas celle de ce plateau (comme l'invalidation par plateau
//! d'Orca). Les jobs antérieurs au lot 42 n'ont pas de signature (jamais
//! « à jour »).

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Préfixe de version du calcul (changer de calcul = changer de préfixe).
pub const SIGNATURE_VERSION: &str = "v1";

/// Clé de l'instantané de scène d'un job qui porte sa signature.
pub const SCENE_KEY: &str = "sliceSignature";

/// Plateaux de la scène : tableau `plates` ou document `{ plates: [...] }`.
fn plates(scene: &Value) -> Option<&Vec<Value>> {
    match scene.get("plates") {
        Some(Value::Array(plates)) => Some(plates),
        Some(Value::Object(doc)) => doc.get("plates").and_then(Value::as_array),
        _ => None,
    }
}

/// Plateau de chaque objet (même règle que le tranchage : absent = 0).
fn object_plate(scene: &Value, object_id: Option<&str>) -> i64 {
    let Some(id) = object_id else { return 0 };
    plates(scene)
        .into_iter()
        .flatten()
        .enumerate()
        .find(|(_, plate)| {
            plate
                .get("objectIds")
                .and_then(Value::as_array)
                .is_some_and(|ids| ids.iter().any(|v| v.as_str() == Some(id)))
        })
        .map(|(index, _)| index as i64)
        .unwrap_or(0)
}

/// Signature du plateau `plate` tranché avec `settings` (config effective) et
/// la scène `scene`.
pub fn plate_signature(settings: &Value, scene: &Value, plate: i64) -> String {
    let objects: Vec<Value> = scene
        .get("objects")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|o| object_plate(scene, o.get("id").and_then(Value::as_str)) == plate)
        .cloned()
        .collect();
    let custom = plates(scene)
        .and_then(|p| p.get(plate.max(0) as usize))
        .and_then(|p| p.get("customGcodes"))
        .cloned()
        .unwrap_or(Value::Null);
    let input = serde_json::json!({
        "plate": plate,
        "settings": settings,
        "objects": objects,
        "customGcodes": custom,
        "calibration": scene.get("calibration").cloned().unwrap_or(Value::Null),
    });
    let mut text = String::new();
    canonical(&input, &mut text);
    let digest = Sha256::digest(text.as_bytes());
    let mut out = String::with_capacity(SIGNATURE_VERSION.len() + 65);
    out.push_str(SIGNATURE_VERSION);
    out.push(':');
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Signature mémorisée dans l'instantané de scène d'un job (lot 42+).
pub fn of_job_scene(scene: Option<&Value>) -> Option<String> {
    scene?
        .get(SCENE_KEY)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// JSON canonique : clés triées, sans blancs.
pub fn canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                canonical(&map[key], out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scene() -> Value {
        json!({
            "objects": [
                { "id": "a", "modelId": "m1", "transform": { "position": [10.0, 20.0, 0.0] } },
                { "id": "b", "modelId": "m2", "transform": { "position": [50.0, 20.0, 0.0] } },
                { "id": "c", "modelId": "m3" }
            ],
            "plates": { "activeId": "p1", "plates": [
                { "id": "p1", "objectIds": ["a", "c"] },
                { "id": "p2", "objectIds": ["b"], "customGcodes": [{ "printZ": 2.0, "type": "pause" }] }
            ] },
            "camera": { "zoom": 1 }
        })
    }

    #[test]
    fn signature_is_stable_and_key_order_independent() {
        let settings = json!({ "wall_loops": "3", "layer_height": "0.2" });
        let reordered = json!({ "layer_height": "0.2", "wall_loops": "3" });
        let s = plate_signature(&settings, &scene(), 0);
        assert!(s.starts_with("v1:") && s.len() == 67, "{s}");
        assert_eq!(s, plate_signature(&reordered, &scene(), 0));
        let mut ui_only = scene();
        ui_only["camera"] = json!({ "zoom": 3 });
        ui_only["plates"]["activeId"] = json!("p2");
        assert_eq!(s, plate_signature(&settings, &ui_only, 0));
    }

    #[test]
    fn signature_follows_settings_and_plate_content_only() {
        let settings = json!({ "wall_loops": "3" });
        let base0 = plate_signature(&settings, &scene(), 0);
        let base1 = plate_signature(&settings, &scene(), 1);
        assert_ne!(base0, base1);
        assert_ne!(
            base0,
            plate_signature(&json!({ "wall_loops": "5" }), &scene(), 0)
        );
        // Objet du plateau 2 déplacé : plateau 1 inchangé, plateau 2 changé.
        let mut moved = scene();
        moved["objects"][1]["transform"]["position"][0] = json!(60.0);
        assert_eq!(base0, plate_signature(&settings, &moved, 0));
        assert_ne!(base1, plate_signature(&settings, &moved, 1));
        // Changement de couleur / pause du plateau 2.
        let mut paused = scene();
        paused["plates"]["plates"][1]["customGcodes"][0]["printZ"] = json!(3.0);
        assert_ne!(base1, plate_signature(&settings, &paused, 1));
        // Paramètres de calibration.
        let mut calib = scene();
        calib["calibration"] = json!({ "mode": "vfa_tower" });
        assert_ne!(base0, plate_signature(&settings, &calib, 0));
    }

    #[test]
    fn canonical_json_sorts_keys() {
        let mut out = String::new();
        canonical(
            &json!({ "b": [1, 2.5, "é"], "a": { "z": null, "y": true } }),
            &mut out,
        );
        assert_eq!(out, r#"{"a":{"y":true,"z":null},"b":[1,2.5,"é"]}"#);
    }

    #[test]
    fn job_scene_signature_is_read_back() {
        assert_eq!(
            of_job_scene(Some(&json!({ "sliceSignature": "v1:ab" }))),
            Some("v1:ab".into())
        );
        assert_eq!(of_job_scene(Some(&json!({}))), None);
        assert_eq!(of_job_scene(None), None);
    }
}
