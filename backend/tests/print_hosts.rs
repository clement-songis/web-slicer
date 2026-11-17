//! Hôtes d'impression comme Orca (adapters::print_host) contre des serveurs
//! factices qui enregistrent les requêtes : séquence, méthodes, chemins,
//! en-têtes et champs identiques à src/slic3r/Utils/*.cpp d'Orca 2.4.1.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use backend::adapters::print_host::{HostClient, HostConfig, HostKind};

#[derive(Debug, Clone)]
struct Seen {
    method: String,
    uri: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl Seen {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    /// Champ multipart « name » → valeur texte.
    fn field(&self, name: &str) -> Option<String> {
        let marker = format!("name=\"{name}\"");
        let at = self.body.find(&marker)?;
        let rest = &self.body[at..];
        let start = rest.find("\r\n\r\n")? + 4;
        let end = rest[start..].find("\r\n--")?;
        Some(rest[start..start + end].to_string())
    }
    fn file_name(&self, field: &str) -> Option<String> {
        let marker = format!("name=\"{field}\"; filename=\"");
        let at = self.body.find(&marker)? + marker.len();
        Some(self.body[at..].split('"').next()?.to_string())
    }
}

type Log = Arc<Mutex<Vec<Seen>>>;

/// Serveur factice : \`respond(method, path_and_query, headers)\` → (statut, corps).
async fn fake(
    respond: impl Fn(&str, &str, &[(String, String)]) -> (u16, String) + Send + Sync + 'static,
) -> (String, Log) {
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let respond = Arc::new(respond);
    let l = log.clone();
    let app = Router::new().fallback(move |req: Request<Body>| {
        let l = l.clone();
        let respond = respond.clone();
        async move {
            let method = req.method().to_string();
            let uri = req
                .uri()
                .path_and_query()
                .map(|p| p.to_string())
                .unwrap_or_default();
            let headers: Vec<(String, String)> = req
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
                .collect();
            let body = to_bytes(req.into_body(), usize::MAX).await.unwrap();
            let (status, text) = respond(&method, &uri, &headers);
            l.lock().unwrap().push(Seen {
                method,
                uri,
                headers,
                body: String::from_utf8_lossy(&body).into_owned(),
            });
            Response::builder()
                .status(StatusCode::from_u16(status).unwrap())
                .body(Body::from(text))
                .unwrap()
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("127.0.0.1:{}", addr.port()), log)
}

fn cfg(host: &str) -> HostConfig {
    HostConfig {
        host: host.into(),
        start_delay: Duration::from_millis(10),
        mks_console_port: 8080,
        ..Default::default()
    }
}

fn seen(log: &Log) -> Vec<Seen> {
    log.lock().unwrap().clone()
}

const GCODE: &[u8] = b"G28\nG1 X10\n";

#[tokio::test]
async fn octoprint_and_astrobox_like_orca() {
    for (kind, text) in [
        (HostKind::Octoprint, "OctoPrint 1.9.3"),
        (HostKind::Astrobox, "AstroBox 1.4"),
    ] {
        let t = text.to_string();
        let (host, log) = fake(move |_, uri, _| {
            if uri == "/api/version" {
                (
                    200,
                    format!(r#"{{"api":"0.1","server":"1.9.3","text":"{t}"}}"#),
                )
            } else {
                (201, "{}".into())
            }
        })
        .await;
        let client = HostClient::new(
            kind,
            HostConfig {
                api_key: Some("CLE".into()),
                ..cfg(&host)
            },
        );
        let info = client.test().await.unwrap();
        assert_eq!(info.version.as_deref(), Some(text));
        let up = client
            .upload("Cube_PLA_9m.gcode", GCODE.to_vec(), true, None)
            .await
            .unwrap();
        assert!(up.print_started);
        let s = seen(&log);
        assert_eq!(
            s.iter()
                .map(|r| (r.method.as_str(), r.uri.as_str()))
                .collect::<Vec<_>>(),
            [
                ("GET", "/api/version"),
                ("GET", "/api/version"),
                ("POST", "/api/files/local")
            ]
        );
        assert!(s.iter().all(|r| r.header("X-Api-Key") == Some("CLE")));
        let post = &s[2];
        assert_eq!(post.field("print").as_deref(), Some("true"));
        assert_eq!(post.field("path").as_deref(), Some(""));
        assert_eq!(post.file_name("file").as_deref(), Some("Cube_PLA_9m.gcode"));
        assert!(post.body.contains("G1 X10"));
    }
    // Type d'hôte incorrect : refus comme Orca (« Mismatched type of print host »).
    let (host, _) = fake(|_, _, _| (200, r#"{"text":"PrusaLink 0.7.0"}"#.into())).await;
    let err = HostClient::new(HostKind::Octoprint, cfg(&host))
        .test()
        .await
        .unwrap_err();
    assert!(err.to_string().contains("incorrect"), "{err}");
}

#[tokio::test]
async fn prusalink_put_post_storage_and_digest_like_orca() {
    // PrusaLink ≥ 0.7 : PUT api/v1/files/<stockage>/<nom>, en-têtes d'Orca.
    let (host, log) = fake(|m, uri, _| match (m, uri) {
        ("GET", "/api/version") => (200, r#"{"text":"PrusaLink 0.7.2"}"#.into()),
        ("GET", "/api/v1/storage") => (200, r#"{"storage_list":[{"name":"USB","path":"/usb","free_space":1000,"read_only":false,"available":true},{"name":"SD","path":"/sdcard","ro":true,"available":true},{"name":"X","path":"/x","available":false}]}"#.into()),
        _ => (201, String::new()),
    })
    .await;
    let client = HostClient::new(
        HostKind::Prusalink,
        HostConfig {
            api_key: Some("K".into()),
            ..cfg(&host)
        },
    );
    let storages = client.storages().await.unwrap();
    assert_eq!(
        storages.iter().map(|s| s.path.as_str()).collect::<Vec<_>>(),
        ["/usb"]
    );
    let up = client
        .upload("A B.gcode", GCODE.to_vec(), true, Some("/usb"))
        .await
        .unwrap();
    assert_eq!(up.path, "/usb/A B.gcode");
    let put = seen(&log).into_iter().find(|r| r.method == "PUT").unwrap();
    assert_eq!(put.uri, "/api/v1/files/usb/A%20B.gcode");
    assert_eq!(put.header("Print-After-Upload"), Some("?1"));
    assert_eq!(put.header("Overwrite"), Some("?1"));
    assert_eq!(put.header("Content-Type"), Some("text/x.gcode"));
    assert_eq!(put.body.as_bytes(), GCODE);

    // Version < 0.7 (ou OctoPrint) : POST api/files/local multipart, champ print.
    let (host, log) = fake(|_, uri, _| {
        if uri == "/api/version" {
            (200, r#"{"text":"OctoPrint 1.3.0"}"#.into())
        } else {
            (201, String::new())
        }
    })
    .await;
    let client = HostClient::new(
        HostKind::Prusalink,
        HostConfig {
            api_key: Some("K".into()),
            ..cfg(&host)
        },
    );
    client
        .upload("c.gcode", GCODE.to_vec(), false, None)
        .await
        .unwrap();
    let post = seen(&log).into_iter().find(|r| r.method == "POST").unwrap();
    assert_eq!(post.uri, "/api/files/local");
    assert_eq!(post.field("print").as_deref(), Some("false"));

    // Authentification utilisateur : digest MD5 en réponse au défi 401.
    let (host, log) = fake(|_, _, headers| {
        let auth = headers
            .iter()
            .find(|(k, _)| k == "authorization")
            .map(|(_, v)| v.clone());
        match auth {
            Some(a) if a.starts_with("Digest ") => (200, r#"{"text":"PrusaLink 0.7.2"}"#.into()),
            _ => (401, String::new()),
        }
    })
    .await;
    // Le défi est servi via WWW-Authenticate : serveur dédié pour l'en-tête.
    drop((host, log));
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let l = log.clone();
    let app = Router::new().fallback(move |req: Request<Body>| {
        let l = l.clone();
        async move {
            let auth = req
                .headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            l.lock().unwrap().push(Seen {
                method: req.method().to_string(),
                uri: req.uri().to_string(),
                headers: auth
                    .clone()
                    .map(|a| vec![("authorization".into(), a)])
                    .unwrap_or_default(),
                body: String::new(),
            });
            match auth {
                Some(a) if a.contains(r#"username="maker""#) && a.contains("response=") => {
                    Response::builder()
                        .status(200)
                        .body(Body::from(r#"{"text":"PrusaLink 0.7.2"}"#))
                        .unwrap()
                }
                _ => Response::builder()
                    .status(401)
                    .header(
                        "WWW-Authenticate",
                        r#"Digest realm="Printer API", nonce="abc123", qop="auth""#,
                    )
                    .body(Body::empty())
                    .unwrap(),
            }
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = HostClient::new(
        HostKind::Prusalink,
        HostConfig {
            auth_user: true,
            user: Some("maker".into()),
            password: Some("secret".into()),
            ..cfg(&format!("127.0.0.1:{port}"))
        },
    );
    assert_eq!(
        client.test().await.unwrap().version.as_deref(),
        Some("PrusaLink 0.7.2")
    );
    let s = seen(&log);
    assert_eq!(s.len(), 2, "défi puis réponse digest");
    assert!(s[1]
        .header("authorization")
        .unwrap()
        .contains(r#"uri="/api/version""#));
}

#[tokio::test]
async fn prusaconnect_post_fields_like_orca() {
    let (host, log) = fake(|_, uri, _| {
        if uri == "/api/version" {
            (200, r#"{"text":"OctoPrint 1.3.10"}"#.into())
        } else {
            (200, "ok".into())
        }
    })
    .await;
    let client = HostClient::new(
        HostKind::Prusaconnect,
        HostConfig {
            api_key: Some("T".into()),
            ..cfg(&host)
        },
    );
    client
        .upload("p.gcode", GCODE.to_vec(), true, None)
        .await
        .unwrap();
    let post = seen(&log).into_iter().find(|r| r.method == "POST").unwrap();
    assert_eq!(post.field("to_print").as_deref(), Some("True"));
    assert!(post.field("print").is_none());
    assert!(post.header("Accept-Language").is_some());
}

#[tokio::test]
async fn duet_rrf_and_dsf_like_orca() {
    // RRF : rr_connect (mot de passe « reprap » par défaut), rr_upload, M32, rr_disconnect.
    let (host, log) = fake(|_, _, _| (200, r#"{"err":0}"#.into())).await;
    let client = HostClient::new(HostKind::Duet, cfg(&host));
    client
        .upload("d e.gcode", GCODE.to_vec(), true, None)
        .await
        .unwrap();
    let s = seen(&log);
    let paths: Vec<String> = s
        .iter()
        .map(|r| format!("{} {}", r.method, r.uri.split("&time=").next().unwrap()))
        .collect();
    assert_eq!(paths[0], "GET /rr_connect?password=reprap");
    assert_eq!(paths[1], "POST /rr_upload?name=0:/gcodes/d%20e.gcode");
    assert_eq!(
        paths[2],
        "GET /rr_gcode?gcode=M32%20%220:/gcodes/d%20e.gcode%22"
    );
    assert_eq!(paths[3], "GET /rr_disconnect");
    assert!(s[0].uri.contains("&time=") && s[1].uri.contains("&time="));
    assert_eq!(s[1].body.as_bytes(), GCODE);
    // Mot de passe refusé (err 1).
    let (host, _) = fake(|_, _, _| (200, r#"{"err":1}"#.into())).await;
    let err = HostClient::new(HostKind::Duet, cfg(&host))
        .test()
        .await
        .unwrap_err();
    assert!(err.to_string().contains("mot de passe"));

    // DSF : rr_connect en échec HTTP → machine/status ; PUT machine/file/gcodes, POST machine/code.
    let (host, log) = fake(|m, uri, _| match (m, uri.split('?').next().unwrap()) {
        ("GET", "/rr_connect") => (404, String::new()),
        ("PUT", _) => (201, String::new()),
        _ => (200, "{}".into()),
    })
    .await;
    HostClient::new(HostKind::Duet, cfg(&host))
        .upload("f.gcode", GCODE.to_vec(), true, None)
        .await
        .unwrap();
    let s = seen(&log);
    let calls: Vec<String> = s
        .iter()
        .map(|r| format!("{} {}", r.method, r.uri.split('?').next().unwrap()))
        .collect();
    assert_eq!(
        calls,
        [
            "GET /rr_connect",
            "GET /machine/status",
            "PUT /machine/file/gcodes/f.gcode",
            "POST /machine/code"
        ]
    );
    assert_eq!(s[3].body, r#"M32 "0:/gcodes/f.gcode""#);
}

#[tokio::test]
async fn repetier_like_orca() {
    let (host, log) = fake(|_, uri, _| match uri {
        "/printer/info" => (
            200,
            r#"{"name":"Repetier-Server","software":"Repetier-Server","version":"1.4.0"}"#.into(),
        ),
        "/printer/list" => (
            200,
            r#"{"data":[{"slug":"Prusa_i3"},{"slug":"Voron"}]}"#.into(),
        ),
        "/printer/api/Voron" => (200, r##"{"groupNames":["#","Pièces"]}"##.into()),
        _ => (200, "{}".into()),
    })
    .await;
    let client = HostClient::new(
        HostKind::Repetier,
        HostConfig {
            api_key: Some("R".into()),
            port: Some("Voron".into()),
            ..cfg(&host)
        },
    );
    assert_eq!(client.printers().await.unwrap(), ["Prusa_i3", "Voron"]);
    assert_eq!(client.groups().await.unwrap(), ["Default", "Pièces"]);
    client
        .upload("r.gcode", GCODE.to_vec(), true, Some("Pièces"))
        .await
        .unwrap();
    client
        .upload("m.gcode", GCODE.to_vec(), false, Some("Default"))
        .await
        .unwrap();
    let s = seen(&log);
    let groups_req = s.iter().find(|r| r.uri == "/printer/api/Voron").unwrap();
    assert_eq!(groups_req.field("a").as_deref(), Some("listModelGroups"));
    let job = s.iter().find(|r| r.uri == "/printer/job/Voron").unwrap();
    assert_eq!(job.field("a").as_deref(), Some("upload"));
    assert_eq!(job.field("group").as_deref(), Some("Pièces"));
    assert_eq!(job.field("name").as_deref(), Some("r.gcode"));
    assert_eq!(job.field("autostart").as_deref(), Some("true"));
    assert_eq!(job.file_name("filename").as_deref(), Some("r.gcode"));
    let model = s.iter().find(|r| r.uri == "/printer/model/Voron").unwrap();
    assert!(
        model.field("group").is_none(),
        "groupe « Default » non envoyé"
    );
    assert!(model.field("autostart").is_none());
    assert!(s.iter().all(|r| r.header("X-Api-Key") == Some("R")));
}

#[tokio::test]
async fn mks_like_orca() {
    // Console TCP (TCPConsole : commande + « \n », réponse « ok ») et upload HTTP.
    let console = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let console_port = console.local_addr().unwrap().port();
    let commands: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let c = commands.clone();
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        loop {
            let (sock, _) = console.accept().await.unwrap();
            let c = c.clone();
            tokio::spawn(async move {
                let (r, mut w) = sock.into_split();
                let mut lines = BufReader::new(r).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    c.lock().unwrap().push(line);
                    w.write_all(b"echo:busy\nok\n").await.unwrap();
                }
            });
        }
    });
    let (host, log) = fake(|_, _, _| (200, r#"{"err":0}"#.into())).await;
    let client = HostClient::new(
        HostKind::Mks,
        HostConfig {
            mks_console_port: console_port,
            ..cfg(&host)
        },
    );
    client.test().await.unwrap();
    client
        .upload("k.gcode", GCODE.to_vec(), true, None)
        .await
        .unwrap();
    let s = seen(&log);
    assert_eq!(s[0].method, "POST");
    assert_eq!(s[0].uri, "/upload?X-Filename=k.gcode");
    assert_eq!(s[0].body.as_bytes(), GCODE);
    assert_eq!(*commands.lock().unwrap(), ["M105", "M23 k.gcode", "M24"]);
}

#[tokio::test]
async fn flashair_like_orca() {
    let (host, log) = fake(|_, uri, _| {
        if uri.starts_with("/command.cgi") {
            (200, "1".into())
        } else {
            (200, "SUCCESS".into())
        }
    })
    .await;
    let client = HostClient::new(HostKind::Flashair, cfg(&host));
    client.test().await.unwrap();
    let up = client
        .upload("f.gcode", GCODE.to_vec(), true, None)
        .await
        .unwrap();
    assert!(!up.print_started, "FlashAir ne démarre pas d'impression");
    let s = seen(&log);
    assert_eq!(s[0].uri, "/command.cgi?op=118");
    assert!(s[1].uri.starts_with("/upload.cgi?WRITEPROTECT=ON&FTIME=0x"));
    assert_eq!(s[2].uri, "/upload.cgi?UPDIR=/");
    assert_eq!(
        (s[3].method.as_str(), s[3].uri.as_str()),
        ("POST", "/upload.cgi")
    );
    assert_eq!(s[3].file_name("file").as_deref(), Some("f.gcode"));
    // Téléversement désactivé sur la carte : test en échec.
    let (host, _) = fake(|_, _, _| (200, "0".into())).await;
    assert!(HostClient::new(HostKind::Flashair, cfg(&host))
        .test()
        .await
        .is_err());
}

#[tokio::test]
async fn esp3d_like_orca() {
    let (host, log) = fake(|_, _, _| (200, "ok".into())).await;
    let client = HostClient::new(HostKind::Esp3d, cfg(&host));
    client.test().await.unwrap();
    let up = client
        .upload("Benchy_PLA_1h2m.gcode", GCODE.to_vec(), true, None)
        .await
        .unwrap();
    assert_eq!(up.path, "Benchy_P.gco", "nom 8.3");
    let s = seen(&log);
    let calls: Vec<String> = s
        .iter()
        .map(|r| format!("{} {}", r.method, r.uri))
        .collect();
    assert_eq!(
        calls,
        [
            "GET /command?plain=M105",
            "POST /upload_serial",
            "GET /command?plain=M23%20Benchy_P.gco",
            "GET /command?plain=M24"
        ]
    );
    assert_eq!(s[1].file_name("file").as_deref(), Some("Benchy_P.gco"));
    assert_eq!(s[1].header("Connection"), Some("keep-alive"));
}
