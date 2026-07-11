//! Pool de workers de tranchage (T063, R9).
//!
//! Boucle de travail au-dessus de `JobRepo` : chaque worker réclame le prochain
//! job `queued` de façon transactionnelle (`claim_next` — jamais deux workers
//! sur le même job), l'exécute via un `JobRunner` abstrait (le vrai runner FFI
//! est câblé en T064/T066), relaie la progression, puis marque le job terminé.
//! Au démarrage, `requeue_running` repasse les jobs `running` orphelins (crash /
//! reboot) en `queued` pour reprise. L'annulation « kill » propage un signal
//! coopératif au runner en cours.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::domain::repo::{JobOutcome, Storage};
use crate::domain::{GcodeId, JobId, SlicingJob};

/// Signal d'annulation coopératif partagé avec le runner d'un job.
#[derive(Clone, Default)]
pub struct Cancel {
    flag: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl Cancel {
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Déclenche l'annulation (réveille les runners en attente sur `cancelled`).
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Se résout dès que l'annulation est demandée (à sélectionner dans le runner).
    pub async fn cancelled(&self) {
        while !self.is_cancelled() {
            let waiter = self.notify.notified();
            if self.is_cancelled() {
                return;
            }
            waiter.await;
        }
    }
}

/// Issue de l'exécution d'un job par le runner.
pub enum RunOutcome {
    Succeeded(GcodeId),
    Failed(serde_json::Value),
    Cancelled,
}

/// Contexte fourni au runner : progression et signal d'annulation.
pub struct JobContext {
    storage: Arc<dyn Storage>,
    job_id: JobId,
    cancel: Cancel,
}

impl JobContext {
    /// Relaie la progression (0–1) et la phase courante (persistées, relayées WS en T065).
    pub async fn report(&self, progress: f64, phase: &str) {
        let _ = self
            .storage
            .jobs()
            .update_progress(self.job_id, progress, phase)
            .await;
    }

    /// Signal d'annulation à surveiller pendant l'exécution.
    pub fn cancel_signal(&self) -> Cancel {
        self.cancel.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}

/// Exécuteur d'un job (le slicing lui-même). Abstrait pour découpler la file du
/// moteur : le runner FFI est branché en T064/T066, les tests injectent un stub.
#[async_trait]
pub trait JobRunner: Send + Sync {
    async fn run(&self, job: SlicingJob, ctx: JobContext) -> RunOutcome;
}

/// Configuration du pool.
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Nombre de workers concurrents.
    pub workers: usize,
    /// Intervalle de scrutation quand la file est vide.
    pub poll_interval: Duration,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            workers: 2,
            poll_interval: Duration::from_millis(200),
        }
    }
}

/// Pool de workers de tranchage.
pub struct Queue {
    storage: Arc<dyn Storage>,
    runner: Arc<dyn JobRunner>,
    config: QueueConfig,
    cancels: Mutex<HashMap<JobId, Cancel>>,
    shutdown: Notify,
    stopping: AtomicBool,
}

/// Poignée de contrôle d'un pool démarré.
pub struct QueueHandle {
    queue: Arc<Queue>,
    workers: Vec<JoinHandle<()>>,
}

impl QueueHandle {
    /// Annule un job en cours (signal coopératif « kill » au runner). La
    /// transition d'état `→ cancelled` reste portée par `JobRepo::cancel`
    /// (chemin API, propriétaire vérifié).
    pub fn cancel(&self, id: JobId) {
        self.queue.cancel(id);
    }

    /// Arrêt propre : stoppe les workers et attend leur fin.
    pub async fn shutdown(self) {
        self.queue.stopping.store(true, Ordering::SeqCst);
        self.queue.shutdown.notify_waiters();
        for w in self.workers {
            let _ = w.await;
        }
    }
}

impl Queue {
    pub fn new(storage: Arc<dyn Storage>, runner: Arc<dyn JobRunner>, config: QueueConfig) -> Self {
        Self {
            storage,
            runner,
            config,
            cancels: Mutex::new(HashMap::new()),
            shutdown: Notify::new(),
            stopping: AtomicBool::new(false),
        }
    }

    /// Reprend les jobs orphelins puis lance les workers.
    pub async fn start(self: Arc<Self>) -> QueueHandle {
        // Reprise au boot : les `running` d'une exécution précédente repassent `queued`.
        let _ = self.storage.jobs().requeue_running().await;

        let workers = (0..self.config.workers)
            .map(|_| {
                let q = Arc::clone(&self);
                tokio::spawn(async move { q.worker_loop().await })
            })
            .collect();

        QueueHandle {
            queue: self,
            workers,
        }
    }

    /// Déclenche l'annulation coopérative d'un job en cours d'exécution.
    pub fn cancel(&self, id: JobId) {
        if let Some(cancel) = self.cancels.lock().unwrap().get(&id) {
            cancel.cancel();
        }
    }

    async fn worker_loop(self: Arc<Self>) {
        loop {
            if self.stopping.load(Ordering::SeqCst) {
                return;
            }
            match self.storage.jobs().claim_next().await {
                Ok(Some(job)) => self.run_job(job).await,
                Ok(None) => {
                    // File vide : attendre l'intervalle ou un signal d'arrêt.
                    tokio::select! {
                        _ = tokio::time::sleep(self.config.poll_interval) => {}
                        _ = self.shutdown.notified() => return,
                    }
                }
                Err(_) => {
                    tokio::time::sleep(self.config.poll_interval).await;
                }
            }
        }
    }

    async fn run_job(&self, job: SlicingJob) {
        let id = job.id;
        let cancel = Cancel::default();
        self.cancels.lock().unwrap().insert(id, cancel.clone());

        let ctx = JobContext {
            storage: Arc::clone(&self.storage),
            job_id: id,
            cancel,
        };
        let outcome = self.runner.run(job, ctx).await;
        self.cancels.lock().unwrap().remove(&id);

        match outcome {
            RunOutcome::Succeeded(gcode_id) => {
                let _ = self
                    .storage
                    .jobs()
                    .finish(id, JobOutcome::Succeeded { gcode_id })
                    .await;
            }
            RunOutcome::Failed(error) => {
                let _ = self
                    .storage
                    .jobs()
                    .finish(id, JobOutcome::Failed { error })
                    .await;
            }
            // L'état `cancelled` est déjà posé par `JobRepo::cancel` (chemin API) ;
            // le worker n'écrase pas cette transition.
            RunOutcome::Cancelled => {}
        }
    }
}
