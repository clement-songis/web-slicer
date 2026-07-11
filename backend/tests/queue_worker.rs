//! Pool de workers de tranchage (T063) : concurrence (jamais 2× le même job),
//! reprise au boot (running orphelin → requeue → traité), annulation « kill »
//! (signal coopératif propagé au runner en cours).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::repo::{NewJob, NewProject, NewUser};
use backend::domain::{JobId, JobStatus, ProjectId, Role, SlicingJob, Storage, UserId};
use backend::queue::{JobContext, JobRunner, Queue, QueueConfig, RunOutcome};
use serde_json::json;

async fn storage() -> (tempfile::TempDir, Arc<SqliteStorage>) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("queue.db").display());
    (dir, Arc::new(SqliteStorage::connect(&url).await.unwrap()))
}

async fn make_project(s: &dyn Storage) -> (UserId, ProjectId) {
    let user = s
        .users()
        .create(NewUser {
            email: format!("{}@test.local", uuid::Uuid::new_v4()),
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

/// Runner instantané : enregistre chaque job vu, puis échoue (état terminal
/// sans dépendre d'un G-code réel).
struct RecordingRunner {
    seen: Arc<Mutex<Vec<JobId>>>,
}

#[async_trait]
impl JobRunner for RecordingRunner {
    async fn run(&self, job: SlicingJob, ctx: JobContext) -> RunOutcome {
        self.seen.lock().unwrap().push(job.id);
        ctx.report(1.0, "terminé").await;
        RunOutcome::Failed(json!({ "stub": true }))
    }
}

async fn wait_until<F, Fut>(mut cond: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..200 {
        if cond().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("condition non atteinte dans le délai imparti");
}

#[tokio::test]
async fn processes_every_job_exactly_once_under_concurrency() {
    let (_d, store) = storage().await;
    let (owner, project) = make_project(store.as_ref()).await;
    enqueue_n(store.as_ref(), owner, project, 20).await;

    let seen = Arc::new(Mutex::new(Vec::new()));
    let runner = Arc::new(RecordingRunner {
        seen: Arc::clone(&seen),
    });
    let queue = Arc::new(Queue::new(
        Arc::clone(&store) as Arc<dyn Storage>,
        runner,
        QueueConfig {
            workers: 4,
            poll_interval: Duration::from_millis(20),
        },
    ));
    let handle = Arc::clone(&queue).start().await;

    // Attendre que les 20 jobs soient dans un état terminal.
    let s2 = Arc::clone(&store);
    wait_until(|| {
        let s = Arc::clone(&s2);
        async move {
            let jobs = s.jobs().list(owner).await.unwrap();
            jobs.iter().all(|j| j.status == JobStatus::Failed)
        }
    })
    .await;
    handle.shutdown().await;

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 20, "chaque job traité une fois");
    let unique: std::collections::HashSet<_> = seen.iter().collect();
    assert_eq!(unique.len(), 20, "aucun job traité deux fois");
}

#[tokio::test]
async fn requeues_orphaned_running_jobs_at_boot() {
    let (_d, store) = storage().await;
    let (owner, project) = make_project(store.as_ref()).await;
    enqueue_n(store.as_ref(), owner, project, 1).await;

    // Simule un crash en cours : le job est réclamé (→ running) mais jamais fini.
    let claimed = store.jobs().claim_next().await.unwrap().unwrap();
    assert_eq!(claimed.status, JobStatus::Running);

    // Un nouveau pool démarre : `requeue_running` le repasse en `queued`, un
    // worker le reprend et le mène à terme.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let queue = Arc::new(Queue::new(
        Arc::clone(&store) as Arc<dyn Storage>,
        Arc::new(RecordingRunner {
            seen: Arc::clone(&seen),
        }),
        QueueConfig {
            workers: 1,
            poll_interval: Duration::from_millis(20),
        },
    ));
    let handle = Arc::clone(&queue).start().await;

    let s2 = Arc::clone(&store);
    wait_until(|| {
        let s = Arc::clone(&s2);
        async move { s.jobs().get(owner, claimed.id).await.unwrap().status == JobStatus::Failed }
    })
    .await;
    handle.shutdown().await;

    assert_eq!(seen.lock().unwrap().as_slice(), &[claimed.id]);
}

/// Runner bloquant : attend l'annulation coopérative avant de rendre la main.
struct CancellableRunner {
    started: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

#[async_trait]
impl JobRunner for CancellableRunner {
    async fn run(&self, _job: SlicingJob, ctx: JobContext) -> RunOutcome {
        self.started.store(true, Ordering::SeqCst);
        ctx.cancel_signal().cancelled().await;
        self.cancelled.store(true, Ordering::SeqCst);
        RunOutcome::Cancelled
    }
}

#[tokio::test]
async fn cancels_a_running_job() {
    let (_d, store) = storage().await;
    let (owner, project) = make_project(store.as_ref()).await;
    enqueue_n(store.as_ref(), owner, project, 1).await;

    let started = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let queue = Arc::new(Queue::new(
        Arc::clone(&store) as Arc<dyn Storage>,
        Arc::new(CancellableRunner {
            started: Arc::clone(&started),
            cancelled: Arc::clone(&cancelled),
        }),
        QueueConfig {
            workers: 1,
            poll_interval: Duration::from_millis(20),
        },
    ));
    let handle = Arc::clone(&queue).start().await;

    // Récupère le job et attend qu'il soit en cours d'exécution.
    let job = store.jobs().list(owner).await.unwrap()[0].clone();
    let started2 = Arc::clone(&started);
    wait_until(move || {
        let s = Arc::clone(&started2);
        async move { s.load(Ordering::SeqCst) }
    })
    .await;

    // Chemin API : transition d'état + signal « kill » au runner.
    store.jobs().cancel(owner, job.id).await.unwrap();
    handle.cancel(job.id);

    let cancelled2 = Arc::clone(&cancelled);
    wait_until(move || {
        let s = Arc::clone(&cancelled2);
        async move { s.load(Ordering::SeqCst) }
    })
    .await;
    handle.shutdown().await;

    assert!(cancelled.load(Ordering::SeqCst), "runner interrompu");
    assert_eq!(
        store.jobs().get(owner, job.id).await.unwrap().status,
        JobStatus::Cancelled
    );
}
