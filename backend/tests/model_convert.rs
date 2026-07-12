//! Service de conversion de modèle (T123) : chemin heureux (décodage → mesh
//! stocké → `set_mesh` → événement `model.converted`) et repli d'échec
//! (`mark_conversion_failed`, aucun événement). Décodeur et dépôt injectés
//! (stubs) : aucun process worker ni base réelle.

use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::broadcast::error::TryRecvError;

use backend::adapters::files::FileStore;
use backend::domain::{
    Model, ModelFormat, ModelId, ModelRepo, NewModel, ProjectId, StorageResult, UserId,
};
use backend::http::dto::ServerEvent;
use backend::http::ws::EventHub;
use backend::models::convert::{ConvertError, MeshDecoder, ModelConverter};

/// Décodeur injecté : renvoie des octets fixes en succès, ou une erreur.
struct StubDecoder {
    result: Result<Vec<u8>, ()>,
}

#[async_trait]
impl MeshDecoder for StubDecoder {
    async fn decode(&self, _source: &Path, _format: ModelFormat) -> Result<Vec<u8>, ConvertError> {
        match &self.result {
            Ok(bytes) => Ok(bytes.clone()),
            Err(()) => Err(ConvertError::NoResult),
        }
    }
}

/// Appel enregistré par le dépôt stub.
#[derive(Debug, Clone, PartialEq)]
enum RepoCall {
    SetMesh {
        id: ModelId,
        mesh_path: String,
        triangles: i64,
    },
    Failed {
        id: ModelId,
        error: String,
    },
}

/// Dépôt de modèles stub : n'enregistre que les mutations de conversion.
#[derive(Default)]
struct StubModelRepo {
    calls: Mutex<Vec<RepoCall>>,
}

impl StubModelRepo {
    fn calls(&self) -> Vec<RepoCall> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl ModelRepo for StubModelRepo {
    async fn create(&self, _owner: UserId, _model: NewModel) -> StorageResult<Model> {
        unimplemented!("non utilisé par le service de conversion")
    }
    async fn get(&self, _owner: UserId, _id: ModelId) -> StorageResult<Model> {
        unimplemented!("non utilisé par le service de conversion")
    }
    async fn list(&self, _owner: UserId, _p: Option<ProjectId>) -> StorageResult<Vec<Model>> {
        unimplemented!("non utilisé par le service de conversion")
    }
    async fn delete(&self, _owner: UserId, _id: ModelId) -> StorageResult<()> {
        unimplemented!("non utilisé par le service de conversion")
    }
    async fn set_mesh(
        &self,
        _owner: UserId,
        id: ModelId,
        mesh_path: &str,
        triangle_count: i64,
    ) -> StorageResult<()> {
        self.calls.lock().unwrap().push(RepoCall::SetMesh {
            id,
            mesh_path: mesh_path.to_string(),
            triangles: triangle_count,
        });
        Ok(())
    }
    async fn mark_conversion_failed(
        &self,
        _owner: UserId,
        id: ModelId,
        error: &str,
    ) -> StorageResult<()> {
        self.calls.lock().unwrap().push(RepoCall::Failed {
            id,
            error: error.to_string(),
        });
        Ok(())
    }
}

/// Modèle STEP factice (le décodeur stub ignore le chemin réel).
fn fake_model(user: UserId) -> Model {
    Model {
        id: ModelId::new(),
        user_id: user,
        project_id: None,
        filename: "piece.step".into(),
        format: ModelFormat::Step,
        file_path: "/data/piece.step".into(),
        mesh_path: None,
        size_bytes: 1024,
        triangle_count: 0,
        repair_report: None,
        conversion_error: None,
    }
}

/// WSMh valide (1 triangle) via le codec moteur, comme sortie de décodeur.
fn one_triangle_wsmh() -> Vec<u8> {
    engine::api::TriangleMesh {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![[0, 1, 2]],
    }
    .encode_display()
}

#[tokio::test]
async fn success_stores_mesh_and_publishes_event() {
    let user = UserId::new();
    let files = FileStore::new(tempfile::tempdir().unwrap().path().to_path_buf());
    let repo = Arc::new(StubModelRepo::default());
    let hub = Arc::new(EventHub::new());
    let mut rx = hub.subscribe(user);

    let converter = ModelConverter::new(
        Arc::new(StubDecoder {
            result: Ok(one_triangle_wsmh()),
        }),
        files.clone(),
        repo.clone(),
        hub.clone(),
        2,
    );
    let model = fake_model(user);
    let model_id = model.id;
    converter.convert(model).await;

    // Persistance : set_mesh appelé avec 1 triangle et un chemin de mesh écrit.
    let calls = repo.calls();
    assert_eq!(calls.len(), 1, "un seul appel de persistance : {calls:?}");
    match &calls[0] {
        RepoCall::SetMesh {
            id,
            mesh_path,
            triangles,
        } => {
            assert_eq!(*id, model_id);
            assert_eq!(*triangles, 1);
            assert!(
                Path::new(mesh_path).exists(),
                "le fichier WSMh est écrit : {mesh_path}"
            );
        }
        other => panic!("attendu SetMesh, obtenu {other:?}"),
    }

    // Notification : model.converted diffusé au propriétaire.
    match rx.try_recv() {
        Ok(ServerEvent::ModelConverted {
            model_id: id,
            mesh_url,
        }) => {
            assert_eq!(id, model_id.to_string());
            assert_eq!(mesh_url, format!("/api/models/{model_id}/mesh"));
        }
        other => panic!("attendu ModelConverted, obtenu {other:?}"),
    }
}

#[tokio::test]
async fn failure_marks_conversion_and_emits_no_event() {
    let user = UserId::new();
    let files = FileStore::new(tempfile::tempdir().unwrap().path().to_path_buf());
    let repo = Arc::new(StubModelRepo::default());
    let hub = Arc::new(EventHub::new());
    let mut rx = hub.subscribe(user);

    let converter = ModelConverter::new(
        Arc::new(StubDecoder { result: Err(()) }),
        files,
        repo.clone(),
        hub.clone(),
        2,
    );
    let model = fake_model(user);
    let model_id = model.id;
    converter.convert(model).await;

    let calls = repo.calls();
    assert_eq!(calls.len(), 1, "un seul appel : {calls:?}");
    match &calls[0] {
        RepoCall::Failed { id, error } => {
            assert_eq!(*id, model_id);
            assert!(!error.is_empty(), "message d'erreur persisté");
        }
        other => panic!("attendu Failed, obtenu {other:?}"),
    }

    // Aucun événement de succès en cas d'échec.
    assert!(
        matches!(rx.try_recv(), Err(TryRecvError::Empty)),
        "aucun model.converted en cas d'échec"
    );
}
