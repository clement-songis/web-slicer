//! Cache des presets **système** pour les listes filtrées par compatibilité
//! (`GET /api/presets?kind=&printer=`, lot 41).
//!
//! Avant : chaque liste rechargeait tous les presets du type (≈ 7 000
//! filaments, 7 Mo de JSON `values_json` décodés) puis remontait la chaîne
//! `inherits` de chaque preset. Les presets système ne changent qu'au reseed :
//! ils sont chargés une fois par type, réduits aux champs utiles (méta +
//! `compatible_printers(_condition)`), avec leur `CompatSpec` résolue d'avance.
//! Seuls les presets de l'utilisateur sont lus à chaque requête.
//!
//! Invalidation : la génération `domain::presets::system_generation()`
//! (incrémentée par tout `reseed_system` validé) est relue à chaque accès. Les
//! presets utilisateur (création, modification, suppression, import,
//! détachement) ne sont jamais mis en cache et la compatibilité est évaluée à
//! chaque requête sur la config de l'imprimante active : aucun autre événement
//! n'a besoin d'invalider.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::domain::preset_compat::{self, CompatSpec};
use crate::domain::{Preset, PresetKind, Storage, StorageResult, UserId};

/// Clés de valeurs lues par `preset_compat::resolve_spec`.
const COMPAT_KEYS: [&str; 2] = ["compatible_printers", "compatible_printers_condition"];

/// Presets système d'un type, prêts pour le filtre de compatibilité.
pub struct SystemPresets {
    /// Génération des presets système au moment du chargement.
    pub generation: u64,
    /// Presets système, ordre de `list_by_kind` (même requête que le chemin
    /// complet) ; `values` réduites aux clés de compatibilité.
    pub presets: Vec<Preset>,
    /// Indices dans `presets` par nom, dans l'ordre de `presets`.
    by_name: HashMap<String, Vec<usize>>,
    /// Indice par identifiant (texte UUID).
    by_id: HashMap<String, usize>,
    /// Noms, identifiants et cibles `inherits` des presets système. Un preset
    /// utilisateur dont le nom (ou l'id) y figure peut s'intercaler dans une
    /// chaîne système ou concurrencer un parent : le chemin complet est alors
    /// utilisé pour garantir une réponse identique.
    pub reserved: HashSet<String>,
    /// Compatibilité résolue sur la chaîne système de chaque preset
    /// instanciable (`None` pour les abstraits).
    pub specs: Vec<Option<CompatSpec>>,
    /// Deux presets instanciables portent le même nom : l'ordre SQL des
    /// ex aequo n'est alors pas reproductible, les listes passent par SQL.
    pub instantiable_homonyms: bool,
}

impl SystemPresets {
    fn build(generation: u64, all: Vec<Preset>) -> Self {
        let presets: Vec<Preset> = all
            .into_iter()
            .map(|mut p| {
                let mut reduced = Map::new();
                if let Value::Object(values) = &p.values {
                    for key in COMPAT_KEYS {
                        if let Some(v) = values.get(key) {
                            reduced.insert(key.to_string(), v.clone());
                        }
                    }
                }
                p.values = Value::Object(reduced);
                p
            })
            .collect();
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_id: HashMap<String, usize> = HashMap::new();
        let mut reserved: HashSet<String> = HashSet::new();
        for (i, p) in presets.iter().enumerate() {
            by_name.entry(p.name.clone()).or_default().push(i);
            by_id.insert(p.id.to_string(), i);
            reserved.insert(p.name.clone());
            reserved.insert(p.id.to_string());
            if let Some(parent) = &p.inherits {
                reserved.insert(parent.clone());
            }
        }
        let mut instantiable: HashSet<&str> = HashSet::new();
        let instantiable_homonyms = presets
            .iter()
            .filter(|p| p.instantiation)
            .any(|p| !instantiable.insert(p.name.as_str()));
        let mut out = Self {
            generation,
            presets,
            by_name,
            by_id,
            reserved,
            specs: Vec::new(),
            instantiable_homonyms,
        };
        let specs = out
            .presets
            .iter()
            .map(|p| {
                p.instantiation
                    .then(|| compat_spec(p, |name| out.candidates(name), |id| out.get_id(id)))
            })
            .collect();
        out.specs = specs;
        out
    }

    /// Presets système portant ce nom (ordre de `presets`).
    pub fn candidates(&self, name: &str) -> Option<Vec<&Preset>> {
        self.by_name
            .get(name)
            .map(|ix| ix.iter().map(|&i| &self.presets[i]).collect())
    }

    /// Preset système par identifiant texte.
    pub fn get_id(&self, id: &str) -> Option<&Preset> {
        self.by_id.get(id).map(|&i| &self.presets[i])
    }
}

/// Compatibilité effective de `leaf` : remonte la chaîne `inherits` exactement
/// comme le chemin complet (`pick_parent` parmi les homonymes, sinon parent
/// désigné par id du même type, arrêt sur cycle) puis `resolve_spec`.
pub fn compat_spec<'a>(
    leaf: &'a Preset,
    by_name: impl Fn(&str) -> Option<Vec<&'a Preset>>,
    by_id: impl Fn(&str) -> Option<&'a Preset>,
) -> CompatSpec {
    let empty = Map::new();
    let mut chain: Vec<&Map<String, Value>> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut current: &Preset = leaf;
    loop {
        if !seen.insert(current.id.to_string()) {
            break; // cycle
        }
        chain.push(match &current.values {
            Value::Object(m) => m,
            _ => &empty,
        });
        let Some(parent) = current.inherits.as_deref() else {
            break;
        };
        let named = by_name(parent);
        let next = crate::http::routes::presets::pick_parent(named.as_ref(), current)
            .or_else(|| by_id(parent).filter(|p| p.kind == leaf.kind));
        match next {
            Some(p) => current = p,
            None => break,
        }
    }
    preset_compat::resolve_spec(chain)
}

/// Cache porté par `AppState` (une base par état : des tests parallèles sur
/// des bases distinctes ne se mélangent pas).
#[derive(Default)]
pub struct PresetListCache {
    kinds: Mutex<HashMap<&'static str, Arc<SystemPresets>>>,
    /// Sérialise les (re)constructions : une seule lecture complète par type.
    build: tokio::sync::Mutex<()>,
}

fn kind_key(kind: PresetKind) -> &'static str {
    match kind {
        PresetKind::MachineModel => "machine_model",
        PresetKind::Machine => "machine",
        PresetKind::Filament => "filament",
        PresetKind::Process => "process",
    }
}

impl PresetListCache {
    fn current(&self, kind: PresetKind, generation: u64) -> Option<Arc<SystemPresets>> {
        let kinds = self.kinds.lock().unwrap_or_else(|e| e.into_inner());
        kinds
            .get(kind_key(kind))
            .filter(|s| s.generation == generation)
            .cloned()
    }

    /// Presets système du type, chargés au premier accès ou après un reseed.
    pub async fn system(
        &self,
        storage: &dyn Storage,
        kind: PresetKind,
    ) -> StorageResult<Arc<SystemPresets>> {
        // Génération lue AVANT la lecture : un reseed concurrent l'incrémente
        // après son commit, l'entrée est alors reconstruite au prochain accès.
        let generation = crate::domain::presets::system_generation();
        if let Some(hit) = self.current(kind, generation) {
            return Ok(hit);
        }
        let _guard = self.build.lock().await;
        let generation = crate::domain::presets::system_generation();
        if let Some(hit) = self.current(kind, generation) {
            return Ok(hit);
        }
        let started = std::time::Instant::now();
        // Même requête que le chemin complet (`list_by_kind`) avec un compte
        // inexistant : les seuls presets système, dans le même ordre.
        let all = storage
            .presets()
            .list_by_kind(kind, UserId(Uuid::nil()))
            .await?;
        let built = Arc::new(SystemPresets::build(generation, all));
        tracing::debug!(
            kind = kind_key(kind),
            presets = built.presets.len(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "cache des presets système (re)construit"
        );
        self.kinds
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(kind_key(kind), built.clone());
        Ok(built)
    }
}
