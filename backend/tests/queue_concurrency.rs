//! Tests de charge de la file (T073, SC-006) : 10 tranchages simultanés lancés
//! par des comptes différents aboutissent tous, sans mélange de résultats
//! (chaque G-code reste lié à son job et cloisonné à son compte).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::repo::{NewGcode, NewJob, NewProject, NewUser};
use backend::domain::{JobId, JobStatus, ProjectId, Role, SlicingJob, Storage, UserId};
use backend::queue::{JobContext, JobRunner, Queue, QueueConfig, RunOutcome};
use serde_json::json;

async fn storage() -> (tempfile::TempDir, Arc<SqliteStorage>) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("concurrency.db").display());
    (dir, Arc::new(SqliteStorage::connect(&url).await.unwrap()))
}

async fn make_account(s: &dyn Storage, email: &str) -> (UserId, ProjectId) {
    let user = s
        .users()
        .create(NewUser {
            email: email.into(),
            password_hash: "x".into(),
            role: Role::User,
        })
        .await
        .unwrap();
    let project = s
        .projects()
        .create(
            user.id,
            NewProject {
                name: "P".into(),
                scene: json!({}),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
    (user.id, project.id)
}

async fn enqueue_n(s: &dyn Storage, owner: UserId, project: ProjectId, n: i64) {
    for i in 0..n {
        s.jobs()
            .enqueue(
                owner,
                NewJob {
                    project_id: project,
                    plate_index: i,
                    resolved_settings: json!({}),
                },
            )
            .await
            .unwrap();
    }
}

/// Runner qui matérialise un G-code **lié à ce job et à ce compte** : permet de
/// vérifier ensuite l'absence de mélange de résultats entre comptes.
struct SlicingStub {
    storage: Arc<dyn Storage>,
}

#[async_trait]
impl JobRunner for SlicingStub {
    async fn run(&self, job: SlicingJob, ctx: JobContext) -> RunOutcome {
        ctx.report(0.5, "slicing").await;
        let gcode = self
            .storage
            .gcodes()
            .create(
                job.user_id,
                NewGcode {
                    job_id: job.id,
                    file_path: format!("{}.gcode", job.id),
                    preview_path: String::new(),
                    stats: json!({ "plate": job.plate_index }),
                    thumbnails: json!([]),
                },
            )
            .await
            .expect("création du G-code du job");
        RunOutcome::Succeeded {
            gcode_id: gcode.id,
            stats: json!({ "plate": job.plate_index }),
        }
    }
}

async fn wait_until<F, Fut>(mut cond: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..300 {
        if cond().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("condition non atteinte dans le délai imparti");
}

#[tokio::test]
async fn ten_concurrent_slices_across_accounts_all_succeed_without_mixing() {
    let (_d, store) = storage().await;

    // Cinq comptes, deux jobs chacun → 10 tranchages simultanés (SC-006).
    let mut accounts: Vec<(UserId, ProjectId)> = Vec::new();
    for i in 0..5 {
        let acc = make_account(store.as_ref(), &format!("user{i}@test.local")).await;
        enqueue_n(store.as_ref(), acc.0, acc.1, 2).await;
        accounts.push(acc);
    }

    // Pool concurrent (4 workers) au-dessus du runner de tranchage simulé.
    let queue = Arc::new(Queue::new(
        Arc::clone(&store) as Arc<dyn Storage>,
        Arc::new(SlicingStub {
            storage: Arc::clone(&store) as Arc<dyn Storage>,
        }),
        QueueConfig {
            workers: 4,
            poll_interval: Duration::from_millis(10),
        },
    ));
    let handle = Arc::clone(&queue).start().await;

    // Tous les jobs de tous les comptes atteignent `succeeded`.
    let owners: Vec<UserId> = accounts.iter().map(|a| a.0).collect();
    let s2 = Arc::clone(&store);
    wait_until(|| {
        let s = Arc::clone(&s2);
        let owners = owners.clone();
        async move {
            for owner in owners {
                let jobs = s.jobs().list(owner).await.unwrap();
                if jobs.len() != 2 || !jobs.iter().all(|j| j.status == JobStatus::Succeeded) {
                    return false;
                }
            }
            true
        }
    })
    .await;
    handle.shutdown().await;

    // Vérifie l'isolation et l'absence de mélange : chaque job d'un compte porte
    // un G-code appartenant à ce compte, lié à ce même job.
    let mut all_gcodes: Vec<(UserId, JobId, backend::domain::GcodeId)> = Vec::new();
    for (owner, _project) in &accounts {
        let jobs = store.jobs().list(*owner).await.unwrap();
        assert_eq!(jobs.len(), 2);
        for job in jobs {
            let gcode_id = job.gcode_id.expect("job succeeded → G-code");
            let gcode = store.gcodes().get(*owner, gcode_id).await.unwrap();
            assert_eq!(
                gcode.job_id, job.id,
                "G-code lié au bon job (pas de mélange)"
            );
            all_gcodes.push((*owner, job.id, gcode_id));
        }
    }

    // 10 G-codes distincts au total.
    assert_eq!(all_gcodes.len(), 10);
    let unique: std::collections::HashSet<_> = all_gcodes.iter().map(|(_, _, g)| *g).collect();
    assert_eq!(unique.len(), 10, "aucun G-code partagé entre jobs");

    // Cloisonnement inter-comptes : le G-code d'un compte est introuvable pour un
    // autre (SC-008), donc aucun résultat ne fuit d'un compte à l'autre.
    let (owner_a, _, gcode_a) = all_gcodes[0];
    let other = accounts
        .iter()
        .map(|a| a.0)
        .find(|u| *u != owner_a)
        .unwrap();
    assert!(
        store.gcodes().get(other, gcode_a).await.is_err(),
        "le G-code d'un compte ne doit pas être visible par un autre"
    );
}
