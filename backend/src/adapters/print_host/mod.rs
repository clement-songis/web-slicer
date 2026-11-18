//! Hôtes d'impression comme Orca (src/slic3r/Utils/*.cpp, PrintHost::get_print_host) :
//! mêmes requêtes que les adaptateurs C++ d'Orca, pour OctoPrint, AstroBox,
//! PrusaLink, PrusaConnect, Duet (RRF / DSF), Repetier-Server, MKS, FlashAir et
//! ESP3D, plus Obico, Creality Print et Flashforge (module `extra`, lot 19).
//! Moonraker garde son client dédié (adapters::moonraker), plus riche
//! (état, pause, fichiers…). SimplyPrint (OAuth + serveur intermédiaire),
//! 3DPrinterOS ne sont pas portés. ElegooLink : module `elegoo` (lot 20).
//!
//! Correspondance des réglages Orca : print_host → \`host\`, printhost_apikey →
//! \`api_key\`, printhost_authorization_type / printhost_user / printhost_password
//! (PrusaLink) → \`auth_user\` / \`user\` / \`password\`, printhost_port (imprimante
//! Repetier) → \`port\`.

use std::time::Duration;

mod elegoo;
mod extra;
pub use elegoo::{classify as elegoo_classify, ElegooType};
pub use extra::{creality_model_name, flashforge_filename};

use md5::{Digest, Md5};
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Type d'hôte (valeurs de \`host_type\` d'Orca, PrintHostType).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostKind {
    Moonraker,
    Octoprint,
    Astrobox,
    Prusalink,
    Prusaconnect,
    Duet,
    Repetier,
    Mks,
    Flashair,
    Esp3d,
    Obico,
    Crealityprint,
    Flashforge,
    Elegoolink,
}

impl HostKind {
    pub fn parse(raw: &str) -> Option<Self> {
        serde_json::from_value(Value::String(raw.to_ascii_lowercase())).ok()
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Moonraker => "moonraker",
            Self::Octoprint => "octoprint",
            Self::Astrobox => "astrobox",
            Self::Prusalink => "prusalink",
            Self::Prusaconnect => "prusaconnect",
            Self::Duet => "duet",
            Self::Repetier => "repetier",
            Self::Mks => "mks",
            Self::Flashair => "flashair",
            Self::Esp3d => "esp3d",
            Self::Obico => "obico",
            Self::Crealityprint => "crealityprint",
            Self::Flashforge => "flashforge",
            Self::Elegoolink => "elegoolink",
        }
    }
    /// Nom affiché par Orca (get_name).
    pub fn display(self) -> &'static str {
        match self {
            Self::Moonraker => "Moonraker",
            Self::Octoprint => "OctoPrint",
            Self::Astrobox => "AstroBox",
            Self::Prusalink => "PrusaLink",
            Self::Prusaconnect => "PrusaConnect",
            Self::Duet => "Duet",
            Self::Repetier => "Repetier",
            Self::Mks => "MKS",
            Self::Flashair => "FlashAir",
            Self::Esp3d => "ESP3D",
            Self::Obico => "Obico",
            Self::Crealityprint => "Creality Print",
            Self::Flashforge => "Flashforge",
            Self::Elegoolink => "ElegooLink",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HostConfig {
    pub host: String,
    pub api_key: Option<String>,
    /// PrusaLink : authentification utilisateur/mot de passe (digest) au lieu de la clé.
    pub auth_user: bool,
    pub user: Option<String>,
    pub password: Option<String>,
    /// Repetier : slug de l'imprimante sur le serveur (printhost_port).
    pub port: Option<String>,
    /// Délai fixe avant M23/M24 (MKS, ESP3D : 1,5 s dans Orca) — réglable en test.
    pub start_delay: Duration,
    /// Port de la console TCP MKS (8080 dans Orca) — réglable en test.
    pub mks_console_port: u16,
    /// Flashforge : numéro de série (flashforge_serial_number) ; avec la clé
    /// (code d'accès), active l'API locale HTTP.
    pub serial_number: Option<String>,
    /// Flashforge : gcode_flavor Klipper (~M640 au lieu de ~M650).
    pub klipper: bool,
    /// Flashforge : délai avant ~M29 (3 s dans Orca) — réglable en test.
    pub save_delay: Duration,
    /// printer_model du preset machine (ElegooLink : protocole par modèle).
    pub printer_model: Option<String>,
    /// ElegooLink : pause entre étapes SDCP (1 s dans Orca) — réglable en test.
    pub ws_delay: Duration,
    /// Ports réglables en test (défauts d'Orca si `None`) : API locale
    /// Flashforge 8898, console Flashforge 8899, WebSocket Creality 9999.
    pub flashforge_http_port: Option<u16>,
    pub flashforge_console_port: Option<u16>,
    pub creality_ws_port: Option<u16>,
    /// WebSocket SDCP ElegooLink (3030).
    pub elegoo_ws_port: Option<u16>,
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("hôte injoignable : {0}")]
    Network(String),
    #[error("hôte : HTTP {0} {1}")]
    Status(u16, String),
    #[error("{0}")]
    Protocol(String),
    #[error("type d'hôte d'impression incorrect : {0}")]
    Mismatch(String),
    #[error("opération non prise en charge par {0}")]
    Unsupported(&'static str),
}

pub type HostResult<T> = Result<T, HostError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostInfo {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Storage {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uploaded {
    pub path: String,
    pub print_started: bool,
}

pub struct HostClient {
    kind: HostKind,
    cfg: HostConfig,
    http: reqwest::Client,
}

/// Percent-encodage d'un composant (Http::url_encode d'Orca, curl_easy_escape).
pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Nom 8.3 d'ESP3D (ESP3D::get_short_name).
pub fn esp3d_short_name(filename: &str) -> String {
    let (stem, ext) = match filename.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s, e),
        _ => (filename, ""),
    };
    let stem: String = stem.chars().take(8).collect();
    let ext: String = ext.chars().take(3).collect();
    if ext.is_empty() {
        stem
    } else {
        format!("{stem}.{ext}")
    }
}

/// FTIME de FlashAir (date FAT, FlashAir::timestamp_str).
pub fn flashair_fattime(t: time::OffsetDateTime) -> String {
    let fat = (((t.year() as u32).saturating_sub(1980)) << 25)
        | ((u8::from(t.month()) as u32) << 21)
        | ((t.day() as u32) << 16)
        | ((t.hour() as u32) << 11)
        | ((t.minute() as u32) << 5)
        | ((t.second() as u32) >> 1);
    format!("{fat:#x}")
}

/// Heure du serveur (Orca : localtime du poste ; seule la validité du format compte).
fn now_local() -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc()
}

/// \`time=%Y-%m-%dT%H:%M:%S\` (Duet::timestamp_str).
fn duet_timestamp() -> String {
    let t = now_local();
    format!(
        "time={:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

fn json(body: &str) -> HostResult<Value> {
    serde_json::from_str(body)
        .map_err(|_| HostError::Protocol("réponse de l'hôte illisible".into()))
}

/// Semver « PrusaLink x.y.z » ≥ 0.7.0 → PUT (PrusaLink::version_check).
pub fn prusalink_uses_put(version_text: Option<&str>) -> bool {
    let Some(text) = version_text else {
        return false;
    };
    let parts: Vec<&str> = text.split(' ').collect();
    if parts.len() != 2 || parts[0] != "PrusaLink" {
        return false;
    }
    let nums: Vec<u64> = parts[1]
        .split(|c: char| !c.is_ascii_digit())
        .take(3)
        .map(|n| n.parse().unwrap_or(0))
        .collect();
    let v = (
        nums.first().copied().unwrap_or(0),
        nums.get(1).copied().unwrap_or(0),
    );
    v >= (0, 7)
}

impl HostClient {
    pub fn new(kind: HostKind, cfg: HostConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(600))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("client HTTP");
        Self { kind, cfg, http }
    }

    pub fn kind(&self) -> HostKind {
        self.kind
    }

    /// make_url d'Orca : « http(s)://… » gardé (avec « / » ajouté), sinon « http://host/path ».
    fn url(&self, path: &str) -> String {
        let host = self.cfg.host.trim();
        if host.starts_with("http://") || host.starts_with("https://") {
            if host.ends_with('/') {
                format!("{host}{path}")
            } else {
                format!("{host}/{path}")
            }
        } else {
            format!("http://{host}/{path}")
        }
    }

    /// Hôte nu (MKS, ESP3D : « http://%host% » sans schéma saisi).
    fn bare_host(&self) -> String {
        self.cfg
            .host
            .trim()
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/')
            .to_string()
    }

    fn with_key(&self, rb: RequestBuilder) -> RequestBuilder {
        match self.cfg.api_key.as_deref() {
            Some(k) => rb.header("X-Api-Key", k),
            None => rb.header("X-Api-Key", ""),
        }
    }

    async fn send(rb: RequestBuilder) -> HostResult<(StatusCode, String)> {
        let resp = rb
            .send()
            .await
            .map_err(|e| HostError::Network(e.to_string()))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(HostError::Status(
                status.as_u16(),
                body.chars().take(300).collect(),
            ));
        }
        Ok((status, body))
    }

    /// Requête PrusaLink/PrusaConnect authentifiée : clé API ou digest HTTP
    /// (Http::auth_digest : défi 401 puis réponse MD5).
    ///
    /// Comme libcurl avec `CURLAUTH_DIGEST` (Http.cpp:599) : le digest est une
    /// authentification à plusieurs passes, donc pour une méthode autre que
    /// GET/HEAD le premier envoi est une **négociation sans corps**
    /// (`authneg` de curl : `Content-Length: 0`). Le fichier n'est transmis
    /// qu'une fois, avec l'en-tête `Authorization` calculé sur le défi reçu.
    /// Si le serveur ne réclame pas d'authentification (pas de 401) : GET/HEAD →
    /// réponse rendue ; autre méthode → requête complète renvoyée sans
    /// `Authorization` (curl s'arrêterait sur la négociation vide et le fichier
    /// ne serait jamais envoyé). Le corps reste transmis une seule fois.
    async fn prusa_request(
        &self,
        method: Method,
        url: &str,
        build: impl Fn(RequestBuilder) -> RequestBuilder,
    ) -> HostResult<(StatusCode, String)> {
        if !self.cfg.auth_user {
            return Self::send(build(self.with_key(self.http.request(method, url)))).await;
        }
        let probe = self.http.request(method.clone(), url);
        let probe = if method == Method::GET || method == Method::HEAD {
            build(probe)
        } else {
            probe.header(reqwest::header::CONTENT_LENGTH, "0")
        };
        let first = probe
            .send()
            .await
            .map_err(|e| HostError::Network(e.to_string()))?;
        if first.status() != StatusCode::UNAUTHORIZED {
            if method == Method::GET || method == Method::HEAD {
                return Self::finish(first).await;
            }
            return Self::send(build(self.http.request(method, url))).await;
        }
        let challenge = first
            .headers()
            .get(reqwest::header::WWW_AUTHENTICATE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let auth = digest_authorization(
            &challenge,
            method.as_str(),
            &uri_path(url),
            self.cfg.user.as_deref().unwrap_or_default(),
            self.cfg.password.as_deref().unwrap_or_default(),
        )
        .ok_or_else(|| HostError::Protocol("défi digest absent".into()))?;
        let resp = build(
            self.http
                .request(method, url)
                .header(reqwest::header::AUTHORIZATION, auth),
        )
        .send()
        .await
        .map_err(|e| HostError::Network(e.to_string()))?;
        Self::finish(resp).await
    }

    async fn finish(resp: Response) -> HostResult<(StatusCode, String)> {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(HostError::Status(
                status.as_u16(),
                body.chars().take(300).collect(),
            ));
        }
        Ok((status, body))
    }

    /// Test de connexion (PrintHost::test).
    pub async fn test(&self) -> HostResult<HostInfo> {
        let name = self.kind.display().to_string();
        match self.kind {
            HostKind::Moonraker => Err(HostError::Unsupported("Moonraker (client dédié)")),
            HostKind::Octoprint | HostKind::Astrobox => {
                let (_, body) =
                    Self::send(self.with_key(self.http.get(self.url("api/version")))).await?;
                let v = json(&body)?;
                let text = v.get("text").and_then(Value::as_str);
                let prefix = if self.kind == HostKind::Octoprint {
                    "OctoPrint"
                } else {
                    "AstroBox"
                };
                if text.is_some_and(|t| !t.starts_with(prefix)) {
                    return Err(HostError::Mismatch(text.unwrap_or_default().into()));
                }
                Ok(HostInfo {
                    name,
                    version: text.map(str::to_string),
                })
            }
            HostKind::Prusalink | HostKind::Prusaconnect => {
                let (_, body) = self
                    .prusa_request(Method::GET, &self.url("api/version"), |r| r)
                    .await?;
                let v = json(&body)?;
                let text = v.get("text").and_then(Value::as_str);
                let ok = text
                    .map(|t| t.starts_with("PrusaLink") || t.starts_with("OctoPrint"))
                    .unwrap_or(false);
                if !ok {
                    return Err(HostError::Mismatch(text.unwrap_or("OctoPrint").into()));
                }
                Ok(HostInfo {
                    name,
                    version: text.map(str::to_string),
                })
            }
            HostKind::Duet => {
                let dsf = self.duet_connect().await?;
                if !dsf {
                    self.duet_disconnect().await;
                }
                Ok(HostInfo {
                    name,
                    version: Some(if dsf { "DSF".into() } else { "RRF".into() }),
                })
            }
            HostKind::Repetier => {
                let (_, body) =
                    Self::send(self.with_key(self.http.get(self.url("printer/info")))).await?;
                let v = json(&body)?;
                let text = v.get("name").and_then(Value::as_str);
                let soft = v.get("software").and_then(Value::as_str);
                let ok = match soft {
                    Some(s) => s == "Repetier-Server",
                    None => text.map(|t| t.starts_with("Repetier")).unwrap_or(true),
                };
                if !ok {
                    return Err(HostError::Mismatch(
                        soft.or(text).unwrap_or("Repetier").into(),
                    ));
                }
                Ok(HostInfo {
                    name,
                    version: v.get("version").and_then(Value::as_str).map(str::to_string),
                })
            }
            HostKind::Mks => {
                self.mks_console(&["M105"]).await?;
                Ok(HostInfo {
                    name,
                    version: None,
                })
            }
            HostKind::Flashair => {
                let (_, body) =
                    Self::send(self.http.get(format!("{}?op=118", self.url("command.cgi"))))
                        .await?;
                if !body.starts_with('1') {
                    return Err(HostError::Protocol(
                        "FlashAir : téléversement désactivé (UPLOAD=1 requis)".into(),
                    ));
                }
                Ok(HostInfo {
                    name,
                    version: None,
                })
            }
            HostKind::Obico => self.obico_test().await,
            HostKind::Crealityprint => self.creality_test().await,
            HostKind::Flashforge => self.flashforge_test().await,
            HostKind::Elegoolink => self.elegoo_test().await,
            HostKind::Esp3d => {
                Self::send(
                    self.http
                        .get(format!("http://{}/command?plain=M105", self.bare_host())),
                )
                .await?;
                Ok(HostInfo {
                    name,
                    version: None,
                })
            }
        }
    }

    /// Envoi d'un G-code (PrintHost::upload), démarrage si \`start\` ; \`target\` =
    /// stockage PrusaLink (« /usb »…) ou groupe Repetier.
    pub async fn upload(
        &self,
        filename: &str,
        data: Vec<u8>,
        start: bool,
        target: Option<&str>,
    ) -> HostResult<Uploaded> {
        let target = target.filter(|t| !t.is_empty());
        match self.kind {
            HostKind::Moonraker => Err(HostError::Unsupported("Moonraker (client dédié)")),
            HostKind::Octoprint | HostKind::Astrobox => {
                // Orca : test préalable (validation du type d'hôte) puis POST api/files/local.
                self.test().await?;
                let form = reqwest::multipart::Form::new()
                    .text("print", if start { "true" } else { "false" })
                    .text("path", "")
                    .part(
                        "file",
                        file_part(filename, data, "application/octet-stream")?,
                    );
                Self::send(
                    self.with_key(self.http.post(self.url("api/files/local")))
                        .multipart(form),
                )
                .await?;
                Ok(Uploaded {
                    path: filename.to_string(),
                    print_started: start,
                })
            }
            HostKind::Prusalink | HostKind::Prusaconnect => {
                let (_, body) = self
                    .prusa_request(Method::GET, &self.url("api/version"), |r| r)
                    .await?;
                let text = json(&body)?
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let ok = text
                    .as_deref()
                    .map(|t| t.starts_with("PrusaLink") || t.starts_with("OctoPrint"))
                    .unwrap_or(false);
                if !ok {
                    return Err(HostError::Mismatch(text.unwrap_or_default()));
                }
                let use_put = prusalink_uses_put(text.as_deref());
                let storage = target.unwrap_or("/local");
                let base = format!(
                    "{}{}",
                    if use_put { "api/v1/files" } else { "api/files" },
                    storage
                );
                if use_put {
                    let url = format!("{}/{}", self.url(&base), url_encode(filename));
                    let body = data.clone();
                    self.prusa_request(Method::PUT, &url, |r| {
                        let r = if start {
                            r.header("Print-After-Upload", "?1")
                        } else {
                            r
                        };
                        r.header("Content-Type", "text/x.gcode")
                            .header("Overwrite", "?1")
                            .body(body.clone())
                    })
                    .await?;
                } else {
                    let connect = self.kind == HostKind::Prusaconnect;
                    let url = self.url(&base);
                    let name = filename.to_string();
                    let bytes = data.clone();
                    self.prusa_request(Method::POST, &url, |r| {
                        let mut form = reqwest::multipart::Form::new().text("path", "");
                        if connect {
                            if start {
                                form = form.text("to_print", "True");
                            }
                        } else {
                            form = form.text("print", if start { "true" } else { "false" });
                        }
                        let part =
                            reqwest::multipart::Part::bytes(bytes.clone()).file_name(name.clone());
                        let r = if connect {
                            r.header("Accept-Language", "fr")
                        } else {
                            r
                        };
                        r.multipart(form.part("file", part))
                    })
                    .await?;
                }
                Ok(Uploaded {
                    path: format!("{storage}/{filename}"),
                    print_started: start,
                })
            }
            HostKind::Duet => {
                let dsf = self.duet_connect().await?;
                let result = self.duet_upload(dsf, filename, data, start).await;
                if !dsf {
                    self.duet_disconnect().await;
                }
                result
            }
            HostKind::Repetier => {
                self.test().await?;
                let port = self.cfg.port.clone().unwrap_or_default();
                let path = if start {
                    format!("printer/job/{port}")
                } else {
                    format!("printer/model/{port}")
                };
                let mut form = reqwest::multipart::Form::new();
                if let Some(group) = target.filter(|g| *g != "Default") {
                    form = form.text("group", group.to_string());
                }
                if start {
                    form = form
                        .text("name", filename.to_string())
                        .text("autostart", "true");
                }
                form = form.text("a", "upload").part(
                    "filename",
                    file_part(filename, data, "application/octet-stream")?,
                );
                Self::send(
                    self.with_key(self.http.post(self.url(&path)))
                        .multipart(form),
                )
                .await?;
                Ok(Uploaded {
                    path: filename.to_string(),
                    print_started: start,
                })
            }
            HostKind::Mks => {
                let url = format!(
                    "http://{}/upload?X-Filename={}",
                    self.bare_host(),
                    url_encode(filename)
                );
                let (_, body) = Self::send(self.http.post(url).body(data)).await?;
                if json(&body)
                    .ok()
                    .and_then(|v| v.get("err").and_then(Value::as_i64))
                    .unwrap_or(0)
                    != 0
                {
                    return Err(HostError::Protocol(format!(
                        "MKS : erreur d'envoi ({body})"
                    )));
                }
                if start {
                    tokio::time::sleep(self.cfg.start_delay).await;
                    self.mks_console(&[&format!("M23 {filename}"), "M24"])
                        .await?;
                }
                Ok(Uploaded {
                    path: filename.to_string(),
                    print_started: start,
                })
            }
            HostKind::Flashair => {
                let prepare = format!(
                    "{}?WRITEPROTECT=ON&FTIME={}",
                    self.url("upload.cgi"),
                    flashair_fattime(now_local())
                );
                let (_, body) = Self::send(self.http.get(prepare)).await?;
                if !body.to_ascii_uppercase().contains("SUCCESS") {
                    return Err(HostError::Protocol(format!("FlashAir : {body}")));
                }
                let (_, body) =
                    Self::send(self.http.get(format!("{}?UPDIR=/", self.url("upload.cgi"))))
                        .await?;
                if !body.to_ascii_uppercase().contains("SUCCESS") {
                    return Err(HostError::Protocol(format!("FlashAir : {body}")));
                }
                let form = reqwest::multipart::Form::new().part(
                    "file",
                    file_part(filename, data, "application/octet-stream")?,
                );
                let (_, body) =
                    Self::send(self.http.post(self.url("upload.cgi")).multipart(form)).await?;
                if !body.to_ascii_uppercase().contains("SUCCESS") {
                    return Err(HostError::Protocol(format!("FlashAir : {body}")));
                }
                // FlashAir ne démarre pas d'impression (Orca : pas de post-action).
                Ok(Uploaded {
                    path: format!("/{filename}"),
                    print_started: false,
                })
            }
            HostKind::Obico => self.obico_upload(filename, data, start).await,
            HostKind::Crealityprint => self.creality_upload(filename, data, start).await,
            HostKind::Flashforge => self.flashforge_upload(filename, data, start).await,
            HostKind::Elegoolink => self.elegoo_upload(filename, data, start).await,
            HostKind::Esp3d => {
                let short = esp3d_short_name(filename);
                let form = reqwest::multipart::Form::new()
                    .part("file", file_part(&short, data, "application/octet-stream")?);
                Self::send(
                    self.http
                        .post(format!("http://{}/upload_serial", self.bare_host()))
                        .header("Connection", "keep-alive")
                        .multipart(form),
                )
                .await?;
                if start {
                    tokio::time::sleep(self.cfg.start_delay).await;
                    let host = self.bare_host();
                    Self::send(self.http.get(format!(
                        "http://{host}/command?plain={}",
                        url_encode(&format!("M23 {short}"))
                    )))
                    .await?;
                    Self::send(self.http.get(format!("http://{host}/command?plain=M24"))).await?;
                }
                Ok(Uploaded {
                    path: short,
                    print_started: start,
                })
            }
        }
    }

    /// Stockages PrusaLink (PrusaLink::get_storage : accessibles, inscriptibles, non pleins).
    pub async fn storages(&self) -> HostResult<Vec<Storage>> {
        if !matches!(self.kind, HostKind::Prusalink | HostKind::Prusaconnect) {
            return Err(HostError::Unsupported(self.kind.display()));
        }
        let (_, body) = self
            .prusa_request(Method::GET, &self.url("api/v1/storage"), |r| r)
            .await?;
        let v = json(&body)?;
        let list = v
            .get("storage_list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(list
            .iter()
            .filter(|s| s.get("available").and_then(Value::as_bool).unwrap_or(true))
            .filter(|s| {
                !s.get("read_only")
                    .or_else(|| s.get("ro"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .filter(|s| {
                s.get("free_space")
                    .map(|f| {
                        f.as_i64()
                            .or_else(|| f.as_str().and_then(|x| x.parse().ok()))
                            .unwrap_or(1)
                    })
                    .unwrap_or(1)
                    > 0
            })
            .filter_map(|s| {
                Some(Storage {
                    path: s.get("path")?.as_str()?.to_string(),
                    name: s
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })
            })
            .collect())
    }

    /// Groupes Repetier (Repetier::get_groups ; « # » → « Default »).
    pub async fn groups(&self) -> HostResult<Vec<String>> {
        if self.kind != HostKind::Repetier {
            return Err(HostError::Unsupported(self.kind.display()));
        }
        let port = self.cfg.port.clone().unwrap_or_default();
        // Http::get + form_add d'Orca : curl envoie un POST multipart.
        let form = reqwest::multipart::Form::new().text("a", "listModelGroups");
        let (_, body) = Self::send(
            self.with_key(self.http.post(self.url(&format!("printer/api/{port}"))))
                .multipart(form),
        )
        .await?;
        let v = json(&body)?;
        Ok(v.get("groupNames")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(|g| {
                        if g == "#" {
                            "Default".to_string()
                        } else {
                            g.to_string()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Imprimantes d'un serveur Repetier (Repetier::get_printers : data[].slug).
    pub async fn printers(&self) -> HostResult<Vec<String>> {
        if self.kind == HostKind::Obico {
            return self.obico_printers().await;
        }
        if self.kind != HostKind::Repetier {
            return Err(HostError::Unsupported(self.kind.display()));
        }
        let (_, body) = Self::send(self.with_key(self.http.get(self.url("printer/list")))).await?;
        let v = json(&body)?;
        if let Some(e) = v.get("error").and_then(Value::as_str) {
            return Err(HostError::Protocol(e.to_string()));
        }
        Ok(v.get("data")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|p| p.get("slug").and_then(Value::as_str).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default())
    }

    // --- Duet ---------------------------------------------------------------

    /// Duet::connect : rr_connect (RRF, code « err ») ; en échec HTTP, machine/status (DSF).
    async fn duet_connect(&self) -> HostResult<bool> {
        let password = self
            .cfg
            .api_key
            .clone()
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| "reprap".into());
        let url = format!(
            "{}?password={}&{}",
            self.url("rr_connect"),
            url_encode(&password),
            duet_timestamp()
        );
        match Self::send(self.http.get(url)).await {
            Ok((_, body)) => match json(&body)
                .ok()
                .and_then(|v| v.get("err").and_then(Value::as_i64))
                .unwrap_or(0)
            {
                0 => Ok(false),
                1 => Err(HostError::Protocol("Duet : mot de passe incorrect".into())),
                2 => Err(HostError::Protocol(
                    "Duet : ressources insuffisantes pour une nouvelle connexion".into(),
                )),
                _ => Err(HostError::Protocol("Duet : erreur inconnue".into())),
            },
            Err(_) => {
                Self::send(self.http.get(self.url("machine/status"))).await?;
                Ok(true)
            }
        }
    }

    async fn duet_disconnect(&self) {
        let _ = self.http.get(self.url("rr_disconnect")).send().await;
    }

    async fn duet_upload(
        &self,
        dsf: bool,
        filename: &str,
        data: Vec<u8>,
        start: bool,
    ) -> HostResult<Uploaded> {
        if dsf {
            let url = self.url(&format!("machine/file/gcodes/{}", url_encode(filename)));
            let (status, _) = Self::send(self.http.put(url).body(data)).await?;
            if status != StatusCode::CREATED {
                return Err(HostError::Protocol("Duet : erreur d'envoi".into()));
            }
        } else {
            let url = format!(
                "{}?name=0:/gcodes/{}&{}",
                self.url("rr_upload"),
                url_encode(filename),
                duet_timestamp()
            );
            let (_, body) = Self::send(self.http.post(url).body(data)).await?;
            if json(&body)
                .ok()
                .and_then(|v| v.get("err").and_then(Value::as_i64))
                .unwrap_or(0)
                != 0
            {
                return Err(HostError::Protocol("Duet : erreur d'envoi".into()));
            }
        }
        if start {
            if dsf {
                Self::send(
                    self.http
                        .post(self.url("machine/code"))
                        .body(format!("M32 \"0:/gcodes/{filename}\"")),
                )
                .await?;
            } else {
                let url = format!(
                    "{}?gcode=M32%20\"0:/gcodes/{}\"",
                    self.url("rr_gcode"),
                    url_encode(filename)
                );
                Self::send(self.http.get(url)).await?;
            }
        }
        Ok(Uploaded {
            path: format!("0:/gcodes/{filename}"),
            print_started: start,
        })
    }

    // --- MKS : console TCP (TCPConsole : commande + « \n », attente de « ok ») ----

    async fn mks_console(&self, commands: &[&str]) -> HostResult<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let addr = format!(
            "{}:{}",
            self.bare_host().split(':').next().unwrap_or_default(),
            self.cfg.mks_console_port
        );
        let stream = tokio::time::timeout(
            Duration::from_secs(10),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| HostError::Network(format!("{addr} : délai dépassé")))?
        .map_err(|e| HostError::Network(e.to_string()))?;
        let (read, mut write) = stream.into_split();
        let mut lines = BufReader::new(read).lines();
        for cmd in commands {
            write
                .write_all(format!("{cmd}\n").as_bytes())
                .await
                .map_err(|e| HostError::Network(e.to_string()))?;
            loop {
                let line = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
                    .await
                    .map_err(|_| HostError::Network("MKS : pas de réponse « ok »".into()))?
                    .map_err(|e| HostError::Network(e.to_string()))?
                    .ok_or_else(|| HostError::Network("MKS : connexion fermée".into()))?;
                if line.trim_start().starts_with("ok") {
                    break;
                }
            }
        }
        Ok(())
    }
}

fn file_part(filename: &str, data: Vec<u8>, mime: &str) -> HostResult<reqwest::multipart::Part> {
    reqwest::multipart::Part::bytes(data)
        .file_name(filename.to_string())
        .mime_str(mime)
        .map_err(|e| HostError::Protocol(e.to_string()))
}

fn uri_path(url: &str) -> String {
    reqwest::Url::parse(url)
        .map(|u| match u.query() {
            Some(q) => format!("{}?{q}", u.path()),
            None => u.path().to_string(),
        })
        .unwrap_or_else(|_| "/".into())
}

fn md5_hex(s: &str) -> String {
    Md5::digest(s.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// En-tête Authorization Digest (RFC 7616, MD5 ; qop=auth si proposé), comme
/// libcurl CURLAUTH_DIGEST utilisé par Http::auth_digest d'Orca.
pub fn digest_authorization(
    challenge: &str,
    method: &str,
    uri: &str,
    user: &str,
    password: &str,
) -> Option<String> {
    let rest = challenge.trim().strip_prefix("Digest")?.trim();
    let mut params = std::collections::HashMap::new();
    for part in split_params(rest) {
        if let Some((k, v)) = part.split_once('=') {
            params.insert(
                k.trim().to_ascii_lowercase(),
                v.trim().trim_matches('"').to_string(),
            );
        }
    }
    let realm = params.get("realm")?;
    let nonce = params.get("nonce")?;
    let ha1 = md5_hex(&format!("{user}:{realm}:{password}"));
    let ha2 = md5_hex(&format!("{method}:{uri}"));
    let qop_auth = params
        .get("qop")
        .is_some_and(|q| q.split(',').any(|x| x.trim() == "auth"));
    let mut header =
        format!(r#"Digest username="{user}", realm="{realm}", nonce="{nonce}", uri="{uri}""#);
    if qop_auth {
        let cnonce = uuid::Uuid::new_v4().simple().to_string()[..16].to_string();
        let nc = "00000001";
        let response = md5_hex(&format!("{ha1}:{nonce}:{nc}:{cnonce}:auth:{ha2}"));
        header.push_str(&format!(
            r#", qop=auth, nc={nc}, cnonce="{cnonce}", response="{response}""#
        ));
    } else {
        header.push_str(&format!(
            r#", response="{}""#,
            md5_hex(&format!("{ha1}:{nonce}:{ha2}"))
        ));
    }
    if let Some(opaque) = params.get("opaque") {
        header.push_str(&format!(r#", opaque="{opaque}""#));
    }
    header.push_str(", algorithm=MD5");
    Some(header)
}

fn split_params(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_match_orca() {
        assert_eq!(
            esp3d_short_name("Mon_Projet_PLA_9m55s.gcode"),
            "Mon_Proj.gco"
        );
        assert_eq!(esp3d_short_name("cube"), "cube");
        assert_eq!(url_encode("a b&c/é.gcode"), "a%20b%26c%2F%C3%A9.gcode");
        assert!(prusalink_uses_put(Some("PrusaLink 0.7.0")));
        assert!(!prusalink_uses_put(Some("PrusaLink 0.6.2")));
        assert!(!prusalink_uses_put(Some("OctoPrint 1.8.0")));
        let t = time::Date::from_calendar_date(2026, time::Month::November, 17)
            .unwrap()
            .with_hms(22, 30, 10)
            .unwrap()
            .assume_utc();
        assert_eq!(
            flashair_fattime(t),
            format!(
                "{:#x}",
                (46u32 << 25) | (11 << 21) | (17 << 16) | (22 << 11) | (30 << 5) | 5
            )
        );
    }

    #[test]
    fn digest_response_follows_rfc() {
        // RFC 2617 §3.5 (sans qop).
        let h = digest_authorization(
            r#"Digest realm="testrealm@host.com", nonce="dcd98b7102dd2f0e8b11d0f600bfb0c093", opaque="5ccc069c403ebaf9f0171e9517f40e41""#,
            "GET",
            "/dir/index.html",
            "Mufasa",
            "Circle Of Life",
        )
        .unwrap();
        assert!(
            h.contains(r#"response="670fd8c2df070c60b045671b8b24ff02""#),
            "{h}"
        );
    }
}
