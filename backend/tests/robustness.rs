//! Robustesse (lot 24) : fichiers malformés ou tronqués sur les parseurs
//! exposés (fuzz léger déterministe), limites de taille, worker moteur bloqué
//! ou abandonné. Le serveur doit répondre proprement (jamais de 5xx ni de
//! panique pour une entrée client) et ne laisser aucun process orphelin.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::{ModelId, Storage, UserId};
use backend::engine::{run_worker_at, WorkerError};
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

struct Harness {
    _dir: tempfile::TempDir,
    storage: Arc<AnyStorage>,
    app: Router,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("robust.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    Harness {
        _dir: dir,
        storage,
        app: router(state, session_layer),
    }
}

fn cookie(resp: &Response) -> String {
    resp.headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn send(app: &Router, req: Request<Body>) -> Response {
    app.clone().oneshot(req).await.unwrap()
}

async fn json_body(resp: Response) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn register(app: &Router, email: &str) -> String {
    let resp = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "email": email, "password": "password123" }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    cookie(&resp)
}

async fn create_project(app: &Router, session: &str) -> String {
    let resp = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/projects")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, session)
            .body(Body::from(json!({ "name": "Robustesse" }).to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

fn multipart(uri: &str, session: &str, filename: &str, content: &[u8]) -> Request<Body> {
    let boundary = "----webslicerrobustness";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(header::COOKIE, session)
        .body(Body::from(body))
        .unwrap()
}

/// Générateur pseudo-aléatoire déterministe (xorshift64*) : fuzz reproductible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
    /// Mutation : retournements de bits, octets écrasés, troncature ou
    /// duplication d'un segment.
    fn mutate(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = input.to_vec();
        match self.below(4) {
            0 => {
                for _ in 0..1 + self.below(16) {
                    let i = self.below(out.len());
                    out[i] ^= 1 << self.below(8);
                }
            }
            1 => {
                for _ in 0..1 + self.below(64) {
                    let i = self.below(out.len());
                    out[i] = self.next() as u8;
                }
            }
            2 => out.truncate(self.below(out.len())),
            _ => {
                let a = self.below(out.len());
                let b = (a + self.below(256)).min(out.len());
                let segment = out[a..b].to_vec();
                let at = self.below(out.len());
                out.splice(at..at, segment);
            }
        }
        out
    }
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// STL binaire dont l'en-tête annonce `claimed` triangles mais n'en contient
/// que `present`.
fn lying_binary_stl(claimed: u32, present: usize) -> Vec<u8> {
    let mut out = vec![0u8; 80];
    out.extend_from_slice(&claimed.to_le_bytes());
    for i in 0..present {
        out.extend_from_slice(&[0u8; 12]);
        for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, i as f32]] {
            for c in v {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        out.extend_from_slice(&[0, 0]);
    }
    out
}

fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write;
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        for (name, data) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    buf.into_inner()
}

/// Corpus d'entrées malformées par format : (nom de fichier, contenu).
fn malformed_corpus() -> Vec<(String, Vec<u8>)> {
    let mut rng = Rng(0x5EED_2024_1118);
    let stl = fixture("cube20.stl");
    let step = fixture("cube20.step");
    let obj = fixture("cube20.obj");
    let project = fixture("orca_project.3mf");
    let mut corpus: Vec<(String, Vec<u8>)> = vec![
        ("empty.stl".into(), Vec::new()),
        ("header-only.stl".into(), vec![0u8; 84]),
        ("lying.stl".into(), lying_binary_stl(u32::MAX, 2)),
        ("lying-small.stl".into(), lying_binary_stl(1_000_000, 0)),
        ("nan.stl".into(), {
            let mut s = lying_binary_stl(1, 1);
            s[96..100].copy_from_slice(&f32::NAN.to_le_bytes());
            s
        }),
        ("ascii-garbage.stl".into(), b"solid x\nfacet normal a b c\nouter loop\nvertex 1 2\nendsolid".to_vec()),
        ("random.stl".into(), rng.bytes(4096)),
        ("random.3mf".into(), rng.bytes(4096)),
        ("empty-zip.3mf".into(), zip_with(&[])),
        ("no-model.3mf".into(), zip_with(&[("[Content_Types].xml", b"<Types/>")])),
        ("bad-xml.3mf".into(), zip_with(&[("3D/3dmodel.model", b"<model><resources><object id=\"1\"><mesh><vertices><vertex x=\"a\"")])),
        ("huge-index.3mf".into(), zip_with(&[("3D/3dmodel.model", b"<?xml version=\"1.0\"?><model unit=\"millimeter\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\"><resources><object id=\"1\" type=\"model\"><mesh><vertices><vertex x=\"0\" y=\"0\" z=\"0\"/></vertices><triangles><triangle v1=\"0\" v2=\"4000000000\" v3=\"7\"/></triangles></mesh></object></resources><build><item objectid=\"1\"/></build></model>")])),
        ("truncated.3mf".into(), project[..project.len() / 2].to_vec()),
        ("garbage.step".into(), b"ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1=CARTESIAN_POINT('',(0.,0.,\n".to_vec()),
        ("random.step".into(), rng.bytes(2048)),
        ("truncated.step".into(), step[..step.len() / 3].to_vec()),
        ("garbage.obj".into(), b"v 1 2\nv a b c\nf 1 2 99999999\nf -5 -6 -7\n".to_vec()),
    ];
    for i in 0..6 {
        corpus.push((format!("mutant-{i}.stl"), rng.mutate(&stl)));
        corpus.push((format!("mutant-{i}.3mf"), rng.mutate(&project)));
        corpus.push((format!("mutant-{i}.obj"), rng.mutate(&obj)));
    }
    for i in 0..3 {
        corpus.push((format!("mutant-{i}.step"), rng.mutate(&step)));
    }
    corpus
}

/// Process `engine-worker` vivants dont la ligne de commande contient `marker`
/// (répertoire de travail propre au test : ignore les workers des tests
/// parallèles du même binaire).
fn live_workers_with_arg(marker: &str) -> Vec<u32> {
    live_worker_children()
        .into_iter()
        .filter(|pid| {
            std::fs::read(format!("/proc/{pid}/cmdline"))
                .map(|c| String::from_utf8_lossy(&c).contains(marker))
                .unwrap_or(false)
        })
        .collect()
}

/// Process `engine-worker` vivants (non zombies) dont ce test est le parent.
fn live_worker_children() -> Vec<u32> {
    let me = std::process::id();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| {
            let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
                return false;
            };
            let Some((head, rest)) = stat.rsplit_once(')') else {
                return false;
            };
            let mut fields = rest.split_whitespace();
            let state = fields.next().unwrap_or("");
            let ppid: u32 = fields.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            head.contains("engine-worker") && ppid == me && state != "Z" && state != "X"
        })
        .collect()
}

#[test]
fn stl_parser_survives_fuzzing() {
    let mut rng = Rng(0xF022);
    let seeds = [
        fixture("cube20.stl"),
        fixture("pyramid.stl"),
        lying_binary_stl(3, 3),
    ];
    let mut parsed = 0;
    for i in 0..3000 {
        let input = match i % 5 {
            0 => {
                let n = rng.below(600);
                rng.bytes(n)
            }
            _ => rng.mutate(&seeds[i % seeds.len()]),
        };
        if backend::mesh::parse_stl(&input).is_ok() {
            parsed += 1;
        }
    }
    // Pas d'assertion sur le nombre : le test vérifie l'absence de panique et
    // de blocage (en-têtes mensongers, NaN, tronqués).
    eprintln!("{parsed}/3000 entrées fuzzées acceptées par parse_stl");
    assert!(backend::mesh::parse_stl(&lying_binary_stl(u32::MAX, 2)).is_err());
}

#[test]
fn mesh_decoder_survives_fuzzing() {
    let mut rng = Rng(0xDEC0DE);
    let mesh = backend::mesh::parse_stl(&fixture("cube20.stl")).unwrap();
    let encoded = mesh.encode();
    for i in 0..3000 {
        let input = if i % 4 == 0 {
            let n = rng.below(400);
            rng.bytes(n)
        } else {
            rng.mutate(&encoded)
        };
        let _ = backend::mesh::Mesh::decode(&input);
    }
}

/// Fichiers malformés déposés par l'API : réponse 201 (conversion asynchrone)
/// ou 4xx, jamais 5xx ; toute conversion se termine (succès ou erreur
/// persistée) ; le serveur reste disponible et aucun worker ne traîne.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn malformed_uploads_never_break_the_server() {
    let h = harness().await;
    let session = register(&h.app, "fuzz@test.local").await;
    let project = create_project(&h.app, &session).await;
    let owner: UserId = h
        .storage
        .users()
        .find_by_email("fuzz@test.local")
        .await
        .unwrap()
        .unwrap()
        .id;

    let corpus = malformed_corpus();
    let mut accepted: Vec<(String, ModelId)> = Vec::new();
    let mut rejected = 0;
    for (name, bytes) in &corpus {
        let resp = send(
            &h.app,
            multipart(
                &format!("/api/projects/{project}/models"),
                &session,
                name,
                bytes,
            ),
        )
        .await;
        let status = resp.status();
        assert!(
            status == StatusCode::CREATED || status.is_client_error(),
            "{name} : statut {status} (attendu 201 ou 4xx)"
        );
        if status == StatusCode::CREATED {
            let body = json_body(resp).await;
            let id = body["id"].as_str().unwrap();
            accepted.push((name.clone(), uuid::Uuid::parse_str(id).unwrap().into()));
        } else {
            rejected += 1;
        }
    }

    // Chaque conversion acceptée aboutit : maillage ou erreur persistée.
    let deadline = std::time::Instant::now() + Duration::from_secs(240);
    let mut converted = 0;
    let mut failed = 0;
    for (name, id) in &accepted {
        loop {
            let model = h.storage.models().get(owner, *id).await.unwrap();
            if model.mesh_path.is_some() {
                converted += 1;
                break;
            }
            if model.conversion_error.is_some() {
                failed += 1;
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "{name} : conversion jamais terminée"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        // Le maillage d'un modèle en échec est refusé proprement (4xx).
        let mesh = send(
            &h.app,
            Request::builder()
                .uri(format!("/api/models/{id}/mesh"))
                .header(header::COOKIE, &session)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert!(
            !mesh.status().is_server_error(),
            "{name} : /mesh répond {}",
            mesh.status()
        );
    }
    eprintln!(
        "{} fichiers malformés : {rejected} refusés (4xx), {converted} convertis, {failed} en erreur de conversion",
        corpus.len()
    );

    // Import de projet 3MF corrompu : refus client, pas d'erreur serveur.
    for (name, bytes) in corpus.iter().filter(|(n, _)| n.ends_with(".3mf")) {
        let resp = send(
            &h.app,
            multipart("/api/projects/import", &session, name, bytes),
        )
        .await;
        assert!(
            !resp.status().is_server_error(),
            "import {name} : {}",
            resp.status()
        );
    }

    let health = send(
        &h.app,
        Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(health.status(), StatusCode::OK);
    // Au plus le worker préchauffé en attente (prêt pour la prochaine opération) ;
    // les workers `self-test` du test parallèle de ce binaire sont ignorés.
    let leftovers: Vec<u32> = live_worker_children()
        .into_iter()
        .filter(|pid| {
            !std::fs::read(format!("/proc/{pid}/cmdline"))
                .map(|c| String::from_utf8_lossy(&c).contains("self-test"))
                .unwrap_or(false)
        })
        .collect();
    assert!(leftovers.len() <= 1, "workers restants : {leftovers:?}");
}

/// Corps JSON au-delà de la limite par défaut (2 Mio) : 413, sans lecture
/// intégrale ni erreur serveur.
#[tokio::test]
async fn oversized_json_bodies_are_rejected_with_413() {
    let h = harness().await;
    let session = register(&h.app, "big@test.local").await;
    let project = create_project(&h.app, &session).await;
    let huge = format!(
        "{{\"expected_version\":1,\"scene\":{{\"pad\":\"{}\"}},\"active_presets\":{{}}}}",
        "x".repeat(3 * 1024 * 1024)
    );
    for (method, uri) in [
        ("PUT", format!("/api/projects/{project}")),
        ("PUT", "/api/me/preferences".to_string()),
        ("POST", "/api/presets".to_string()),
    ] {
        let resp = send(
            &h.app,
            Request::builder()
                .method(method)
                .uri(&uri)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &session)
                .body(Body::from(huge.clone()))
                .unwrap(),
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::PAYLOAD_TOO_LARGE,
            "{method} {uri}"
        );
    }
}

fn worker_bin() -> Option<PathBuf> {
    let bin = backend::engine::worker_binary();
    bin.exists().then_some(bin)
}

/// Worker bloqué : tué au délai, aucune trace de process ; futur abandonné
/// (annulation, client parti) : worker tué aussi.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hung_or_abandoned_workers_leave_no_process_behind() {
    let Some(bin) = worker_bin() else {
        eprintln!("engine-worker absent — test ignoré");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().to_string_lossy().to_string();

    let err = run_worker_at(
        &bin,
        &["self-test", "--hang", &work],
        Duration::from_millis(600),
    )
    .await
    .expect_err("délai");
    assert!(matches!(err, WorkerError::Timeout(_)), "{err:?}");

    // Futur abandonné en cours de route (délai interne long, délai externe court).
    let abandoned = tokio::time::timeout(
        Duration::from_millis(600),
        run_worker_at(
            &bin,
            &["self-test", "--hang", &work],
            Duration::from_secs(3600),
        ),
    )
    .await;
    assert!(abandoned.is_err(), "le futur doit avoir été abandonné");

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !live_workers_with_arg(&work).is_empty() && std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let leftovers = live_workers_with_arg(&work);
    assert!(leftovers.is_empty(), "workers orphelins : {leftovers:?}");
}
