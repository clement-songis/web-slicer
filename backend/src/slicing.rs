//! Runner de tranchage réel (T064/T066) : implémente `JobRunner` en pilotant le
//! process moteur isolé `engine-worker slice`.
//!
//! Pipeline d'un job :
//!   1. assemble le `Model` moteur du projet (maillages STL décodés côté serveur,
//!      logique partagée avec l'export 3MF) ;
//!   2. re-résout la configuration depuis les presets actifs du projet (la même
//!      résolution typée que l'export ; le blob `resolved_settings` figé est une
//!      projection JSON lossy, non ré-injectable telle quelle) ;
//!   3. sérialise une `SliceRequest` dans un répertoire de travail isolé et lance
//!      `engine-worker slice <request.json>` (crash C++ contenu, timeout, kill) ;
//!   4. lit le G-code produit, le matérialise (`store_gcode` : fichier + ligne
//!      `gcodes` avec stats/vignettes), et renvoie l'issue avec ses statistiques.
//!
//! L'isolation moteur (process séparé) et l'annulation coopérative sont fournies
//! par l'infra `crate::engine` et la file `crate::queue`.

use std::time::Duration;

use async_trait::async_trait;

use crate::domain::SlicingJob;
use crate::engine::run_worker;
use crate::gcode::store_gcode;
use crate::http::routes::{export::build_stl_model, slice::resolve_active_config};
use crate::http::state::AppState;
use crate::queue::{JobContext, JobRunner, RunOutcome};

/// Délai maximal d'un tranchage (au-delà, le worker est tué). Plus large que le
/// timeout de décodage : un tranchage complet peut être long.
const SLICE_TIMEOUT: Duration = Duration::from_secs(600);

/// Runner FFI branché sur la file (T063). Détient un `AppState` (stockage,
/// fichiers, résolution de presets) — cloné à la construction dans `build_app`.
pub struct SliceRunner {
    state: AppState,
}

impl SliceRunner {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Exécute le tranchage et renvoie (stats client, id du G-code stocké).
    async fn slice(
        &self,
        job: &SlicingJob,
        ctx: &JobContext,
    ) -> anyhow::Result<(serde_json::Value, crate::domain::GcodeId)> {
        ctx.report(0.05, "préparation").await;

        let project = self
            .state
            .storage
            .projects()
            .get(job.user_id, job.project_id)
            .await?;

        // Modèle moteur (STL décodés) — partagé avec l'export 3MF.
        let model = build_stl_model(&self.state, job.user_id, job.project_id)
            .await
            .map_err(|e| anyhow::anyhow!("assemblage du modèle : {e:?}"))?;
        if model.is_empty() {
            anyhow::bail!("aucun objet à trancher (modèles STL absents)");
        }

        // Configuration figée : presets actifs re-résolus (typée).
        let (config, _warnings) =
            resolve_active_config(&self.state, job.user_id, &project.active_presets)
                .await
                .map_err(|e| anyhow::anyhow!("résolution de la configuration : {e:?}"))?;

        // Requête sérialisée dans un répertoire de travail isolé (garantie n°3).
        let work = tempfile::tempdir()?;
        let request = engine::api::SliceRequest {
            model,
            config,
            plate_index: job.plate_index.max(0) as u32,
            work_dir: work.path().to_path_buf(),
        };
        let request_path = work.path().join("request.json");
        std::fs::write(&request_path, serde_json::to_vec(&request)?)?;

        if ctx.is_cancelled() {
            anyhow::bail!("annulé");
        }
        ctx.report(0.15, "tranchage").await;

        // Process moteur isolé : `engine-worker slice <request.json>`.
        let request_arg = request_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("chemin de requête non-UTF8"))?;
        let stdout = run_worker(&["slice", request_arg], SLICE_TIMEOUT).await?;
        let result = parse_slice_result(&stdout)?;

        ctx.report(0.9, "enregistrement").await;

        // Matérialise le G-code (fichier + ligne `gcodes` + stats/vignettes).
        let gcode_text = std::fs::read_to_string(&result.gcode_path)?;
        let gcode = store_gcode(
            &self.state.files,
            self.state.storage.gcodes(),
            job.user_id,
            job.id,
            &gcode_text,
        )
        .await?;

        Ok((gcode.stats, gcode.id))
    }
}

#[async_trait]
impl JobRunner for SliceRunner {
    async fn run(&self, job: SlicingJob, ctx: JobContext) -> RunOutcome {
        if ctx.is_cancelled() {
            return RunOutcome::Cancelled;
        }
        match self.slice(&job, &ctx).await {
            Ok((stats, gcode_id)) => {
                ctx.report(1.0, "terminé").await;
                RunOutcome::Succeeded { gcode_id, stats }
            }
            Err(e) => {
                if ctx.is_cancelled() {
                    return RunOutcome::Cancelled;
                }
                tracing::warn!(job = %job.id, error = %e, "tranchage échoué");
                RunOutcome::Failed(serde_json::json!({ "message": e.to_string() }))
            }
        }
    }
}

/// Décode la sortie du worker : la dernière ligne `R {json}` porte le
/// `SliceResult` (le protocole `P`/`R`/`E` de `engine-worker`).
fn parse_slice_result(stdout: &[u8]) -> anyhow::Result<engine::api::SliceResult> {
    let text = String::from_utf8_lossy(stdout);
    let payload = text
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("R "))
        .ok_or_else(|| anyhow::anyhow!("sortie moteur sans résultat « R »"))?;
    Ok(serde_json::from_str(payload)?)
}
