//! Hôtes portés au lot 19 depuis src/slic3r/Utils d'Orca 2.4.1 :
//! Obico (Obico.cpp), Creality Print (CrealityPrint.cpp) et Flashforge
//! (Flashforge.cpp : API locale HTTP :8898 si numéro de série + code d'accès,
//! sinon console TCP :8899 façon TCPConsole.cpp). Mêmes requêtes, champs et
//! séquences que le C++ ; les ports réseau sont réglables pour les tests.

use std::time::Duration;

use base64::Engine;
use serde_json::{json, Value};

use super::{json, url_encode, HostClient, HostError, HostInfo, HostResult, Uploaded};

/// Ports par défaut d'Orca.
pub const FLASHFORGE_HTTP_PORT: u16 = 8898;
pub const FLASHFORGE_CONSOLE_PORT: u16 = 8899;
pub const CREALITY_WS_PORT: u16 = 9999;

/// Modèles Creality multicouleur (CrealityPrint::supports_multi_color_print).
const CREALITY_MULTI_COLOR: [&str; 4] = ["F008", "F012", "F021", "F022"];

/// Nom de modèle affiché (CrealityPrint::model_name).
pub fn creality_model_name(model: Option<&str>) -> String {
    match model {
        None | Some("") => "unreachable".into(),
        Some("F008") => "K2 Plus".into(),
        Some("F012") => "K2 Pro".into(),
        Some("F021") => "K2".into(),
        Some("F022") => "SPARKX i7".into(),
        Some(m) => format!("unknown ({m})"),
    }
}

/// CrealityPrint::safe_filename : espaces → `_`.
pub fn creality_safe_filename(name: &str) -> String {
    name.replace(' ', "_")
}

/// sanitize_flashforge_filename : nom de base, tout caractère hors
/// `[A-Za-z0-9._-]` → `_` ; vide → « print » + extension de repli.
pub fn flashforge_filename(filename: &str, fallback_extension: &str) -> String {
    let base = filename.rsplit(['/', '\\']).next().unwrap_or_default();
    let base = if base.is_empty() {
        format!("print{fallback_extension}")
    } else {
        base.to_string()
    };
    base.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// validate_local_api_response : JSON objet, `code` (ou `err`) nul.
fn flashforge_validate(body: &str) -> HostResult<Value> {
    let v: Value = serde_json::from_str(body)
        .ok()
        .filter(Value::is_object)
        .ok_or_else(|| {
            HostError::Protocol("Flashforge returned an invalid JSON response.".into())
        })?;
    let as_int = |x: &Value| -> Option<i64> {
        match x {
            Value::Number(n) => n.as_i64(),
            Value::Bool(b) => Some(i64::from(*b)),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        }
    };
    let code = v
        .get("code")
        .and_then(as_int)
        .or_else(|| v.get("err").and_then(as_int));
    if let Some(code) = code.filter(|c| *c != 0) {
        let msg = v
            .get("message")
            .or_else(|| v.get("msg"))
            .and_then(Value::as_str)
            .filter(|m| !m.is_empty())
            .unwrap_or("Request failed");
        return Err(HostError::Protocol(format!(
            "Flashforge local API error {code}: {msg}"
        )));
    }
    Ok(v)
}

/// Commandes console Flashforge (Flashforge.hpp).
const FF_CONTROL: &str = "~M601 S1\r\n";
const FF_DEVICE_INFO: &str = "~M115\r\n";
const FF_CONNECT_KLIPPER: &str = "~M640\r\n";
const FF_CONNECT_LEGACY: &str = "~M650\r\n";
const FF_STATUS: &str = "~M119\r\n";
const FF_SAVE_FILE: &str = "~M29\r\n";
/// Taille des paquets de données (m_bufferSize).
const FF_BUFFER: usize = 4096;

/// Élément de file TCPConsole : commande (ligne + « \n », attend « ok ») ou
/// données brutes (aucune attente).
enum Serial<'a> {
    Command(String),
    Data(&'a [u8]),
}

impl HostClient {
    pub(super) fn host_name(&self) -> String {
        // Http::get_host_from_url / extract_host_name : hôte sans schéma, port ni chemin.
        let h = self.bare_host();
        let h = h.split('/').next().unwrap_or_default();
        match h.rsplit_once(':') {
            Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => host.to_string(),
            _ => h.to_string(),
        }
    }

    fn bearer(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        rb.header(
            "Authorization",
            format!("Bearer {}", self.cfg.api_key.as_deref().unwrap_or_default()),
        )
    }

    // --- Obico ----------------------------------------------------------------

    /// Obico::test : clé requise, `GET api/v1/version/` (Bearer).
    pub(super) async fn obico_test(&self) -> HostResult<HostInfo> {
        if self.cfg.api_key.as_deref().unwrap_or_default().is_empty() {
            return Err(HostError::Protocol("Obico : clé d'API requise".into()));
        }
        let (_, body) = Self::send(self.bearer(self.http.get(self.url("api/v1/version/")))).await?;
        let version = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| v.get("version").and_then(Value::as_str).map(str::to_string));
        Ok(HostInfo {
            name: "Obico".into(),
            version,
        })
    }

    /// Obico::get_printers : `GET api/v1/printers/` → « nom [id] ».
    pub(super) async fn obico_printers(&self) -> HostResult<Vec<String>> {
        let (_, body) =
            Self::send(self.bearer(self.http.get(self.url("api/v1/printers/")))).await?;
        let v = json(&body)?;
        if let Some(e) = v.get("error").and_then(Value::as_str) {
            return Err(HostError::Protocol(e.to_string()));
        }
        let items: Vec<Value> = match &v {
            Value::Array(a) => a.clone(),
            Value::Object(m) => m.values().cloned().collect(),
            _ => Vec::new(),
        };
        items
            .iter()
            .map(|p| {
                let name = p.get("name").and_then(Value::as_str);
                let id = p.get("id").map(|i| match i {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                });
                match (name, id) {
                    (Some(n), Some(i)) => Ok(format!("{n} [{i}]")),
                    _ => Err(HostError::Protocol(
                        "Enumeration of host printers failed.".into(),
                    )),
                }
            })
            .collect()
    }

    /// Obico::upload : test, puis `POST api/v1/g_code_files/` (print, path,
    /// printer_id = printhost_port, filename, file).
    pub(super) async fn obico_upload(
        &self,
        filename: &str,
        data: Vec<u8>,
        start: bool,
    ) -> HostResult<Uploaded> {
        self.obico_test().await?;
        let (parent, name) = split_parent(filename);
        let form = reqwest::multipart::Form::new()
            .text("print", if start { "true" } else { "false" })
            .text("path", parent.to_string())
            .text("printer_id", self.cfg.port.clone().unwrap_or_default())
            .text("filename", name.to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(data).file_name(name.to_string()),
            );
        Self::send(
            self.bearer(self.http.post(self.url("api/v1/g_code_files/")))
                .multipart(form),
        )
        .await?;
        Ok(Uploaded {
            path: filename.to_string(),
            print_started: start,
        })
    }

    // --- Creality Print -------------------------------------------------------

    /// CrealityPrint::test : `GET info` (Bearer, 5 s) ; modèle lu dans `model`.
    async fn creality_info(&self) -> HostResult<Option<String>> {
        let (_, body) = Self::send(
            self.bearer(self.http.get(self.url("info")))
                .timeout(Duration::from_secs(5)),
        )
        .await?;
        Ok(serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| v.get("model").and_then(Value::as_str).map(str::to_string)))
    }

    pub(super) async fn creality_test(&self) -> HostResult<HostInfo> {
        let model = self.creality_info().await?;
        Ok(HostInfo {
            name: "Creality Print".into(),
            version: Some(creality_model_name(model.as_deref())),
        })
    }

    /// CrealityPrint::upload : test, `POST upload/<nom>` (champ `path` sauf
    /// modèles multicouleur, fichier `file`), puis démarrage par WebSocket
    /// (port 9999) si demandé.
    pub(super) async fn creality_upload(
        &self,
        filename: &str,
        data: Vec<u8>,
        start: bool,
    ) -> HostResult<Uploaded> {
        let model = self.creality_info().await?;
        let multi = model
            .as_deref()
            .is_some_and(|m| CREALITY_MULTI_COLOR.contains(&m));
        let (parent, name) = split_parent(filename);
        let safe = creality_safe_filename(name);
        let mut form = reqwest::multipart::Form::new();
        if !multi {
            form = form.text("path", parent.to_string());
        }
        form = form.part(
            "file",
            reqwest::multipart::Part::bytes(data).file_name(safe.clone()),
        );
        Self::send(
            self.bearer(
                self.http
                    .post(self.url(&format!("upload/{}", url_encode(&safe)))),
            )
            .multipart(form),
        )
        .await?;
        if start {
            self.creality_start(&safe, multi).await?;
        }
        Ok(Uploaded {
            path: safe,
            print_started: start,
        })
    }

    /// CrealityPrint::start_print (sans informations de couleurs étendues :
    /// liste de correspondance vide, autotest désactivé, comme Orca sans
    /// `extended_info`).
    async fn creality_start(&self, filename: &str, multi: bool) -> HostResult<()> {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::Message;
        let port = self.cfg.creality_ws_port.unwrap_or(CREALITY_WS_PORT);
        let url = format!("ws://{}:{port}/", self.host_name());
        let (mut ws, _) = tokio::time::timeout(
            Duration::from_secs(5),
            tokio_tungstenite::connect_async(&url),
        )
        .await
        .map_err(|_| HostError::Network(format!("{url} : délai dépassé")))?
        .map_err(|e| HostError::Network(e.to_string()))?;
        let send = |v: Value| Message::Text(v.to_string().into());
        if multi {
            let path = format!("/mnt/UDISK/printer_data/gcodes/{filename}");
            ws.send(send(json!({ "method": "set", "params": { "colorMatch": { "path": path, "list": [] } } })))
                .await
                .map_err(|e| HostError::Network(e.to_string()))?;
            ws.send(send(json!({ "method": "set", "params": { "multiColorPrint": { "gcode": path, "enableSelfTest": 0 } } })))
                .await
                .map_err(|e| HostError::Network(e.to_string()))?;
        } else {
            ws.send(send(json!({ "method": "set", "params": {
                "opGcodeFile": format!("printprt:/usr/data/printer_data/gcodes/{filename}") } })))
                .await
                .map_err(|e| HostError::Network(e.to_string()))?;
            // Orca lit une réponse (délai de réception 3 s), sans l'interpréter.
            let _ = tokio::time::timeout(Duration::from_secs(3), ws.next()).await;
        }
        let _ = ws.close(None).await;
        Ok(())
    }

    // --- Flashforge -----------------------------------------------------------

    fn flashforge_local_api(&self) -> Option<(String, String)> {
        let sn = self.cfg.serial_number.clone().filter(|s| !s.is_empty())?;
        let code = self.cfg.api_key.clone().filter(|s| !s.is_empty())?;
        Some((sn, code))
    }

    fn flashforge_url(&self, path: &str) -> String {
        let port = self
            .cfg
            .flashforge_http_port
            .unwrap_or(FLASHFORGE_HTTP_PORT);
        format!("http://{}:{port}/{path}", self.host_name())
    }

    /// TCPConsole::run_queue : connexion, commandes (attente d'une ligne « ok »)
    /// et données brutes, délai `delay` avant chaque élément.
    async fn flashforge_console(&self, queue: &[Serial<'_>], delay: Duration) -> HostResult<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let port = self
            .cfg
            .flashforge_console_port
            .unwrap_or(FLASHFORGE_CONSOLE_PORT);
        let addr = format!("{}:{port}", self.host_name());
        let stream = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| HostError::Network(format!("{addr} : délai dépassé")))?
        .map_err(|e| HostError::Network(e.to_string()))?;
        let (read, mut write) = stream.into_split();
        let mut lines = BufReader::new(read).lines();
        for item in queue {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            match item {
                Serial::Command(cmd) => {
                    write
                        .write_all(format!("{cmd}\n").as_bytes())
                        .await
                        .map_err(|e| HostError::Network(e.to_string()))?;
                    loop {
                        let line = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
                            .await
                            .map_err(|_| {
                                HostError::Network("Flashforge : pas de réponse « ok »".into())
                            })?
                            .map_err(|e| HostError::Network(e.to_string()))?
                            .ok_or_else(|| {
                                HostError::Network("Flashforge : connexion fermée".into())
                            })?;
                        if line.trim().eq_ignore_ascii_case("ok") {
                            break;
                        }
                    }
                }
                Serial::Data(bytes) => write
                    .write_all(bytes)
                    .await
                    .map_err(|e| HostError::Network(e.to_string()))?,
            }
        }
        let _ = write.shutdown().await;
        Ok(())
    }

    /// Flashforge::test : API locale (`POST detail`) ou console (~M601 S1).
    pub(super) async fn flashforge_test(&self) -> HostResult<HostInfo> {
        if let Some((sn, code)) = self.flashforge_local_api() {
            let (_, body) = Self::send(
                self.http
                    .post(self.flashforge_url("detail"))
                    .header("Content-Type", "application/json")
                    .body(json!({ "serialNumber": sn, "checkCode": code }).to_string()),
            )
            .await?;
            let v = flashforge_validate(&body)?;
            let detail = v.get("detail").unwrap_or(&v);
            return Ok(HostInfo {
                name: "Flashforge".into(),
                version: detail
                    .get("firmwareVersion")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        self.flashforge_console(&[Serial::Command(FF_CONTROL.into())], Duration::ZERO)
            .await?;
        Ok(HostInfo {
            name: "Flashforge".into(),
            version: None,
        })
    }

    /// Flashforge::upload.
    pub(super) async fn flashforge_upload(
        &self,
        filename: &str,
        data: Vec<u8>,
        start: bool,
    ) -> HostResult<Uploaded> {
        let ext = filename
            .rsplit_once('.')
            .map(|(_, e)| format!(".{e}"))
            .unwrap_or_else(|| ".gcode".into());
        let name = flashforge_filename(filename, &ext);
        if let Some((sn, code)) = self.flashforge_local_api() {
            // upload_local_api : en-têtes d'Orca, materialMappings = base64("[]").
            let form = reqwest::multipart::Form::new().part(
                "gcodeFile",
                reqwest::multipart::Part::bytes(data.clone()).file_name(name.clone()),
            );
            let mappings = base64::engine::general_purpose::STANDARD.encode("[]");
            let (_, body) = Self::send(
                self.http
                    .post(self.flashforge_url("uploadGcode"))
                    .header("serialNumber", sn)
                    .header("checkCode", code)
                    .header("fileSize", data.len().to_string())
                    .header("printNow", if start { "true" } else { "false" })
                    .header("levelingBeforePrint", "false")
                    .header("flowCalibration", "false")
                    .header("firstLayerInspection", "false")
                    .header("timeLapseVideo", "false")
                    .header("useMatlStation", "false")
                    .header("gcodeToolCnt", "")
                    .header("materialMappings", mappings)
                    .multipart(form),
            )
            .await?;
            flashforge_validate(&body)?;
            return Ok(Uploaded {
                path: name,
                print_started: start,
            });
        }
        // Console : connect (contrôle, infos, connexion Klipper/legacy, état),
        // puis M28 + données sur une nouvelle connexion, M29 après 3 s, M23.
        let connect = if self.cfg.klipper {
            FF_CONNECT_KLIPPER
        } else {
            FF_CONNECT_LEGACY
        };
        self.flashforge_console(
            &[
                Serial::Command(FF_CONTROL.into()),
                Serial::Command(FF_DEVICE_INFO.into()),
                Serial::Command(connect.into()),
                Serial::Command(FF_STATUS.into()),
            ],
            Duration::ZERO,
        )
        .await?;
        let mut queue = vec![Serial::Command(format!(
            "~M28 {} 0:/user/{name}",
            data.len()
        ))];
        queue.extend(data.chunks(FF_BUFFER).map(Serial::Data));
        self.flashforge_console(&queue, Duration::ZERO).await?;
        self.flashforge_console(&[Serial::Command(FF_SAVE_FILE.into())], self.cfg.save_delay)
            .await?;
        if start {
            self.flashforge_console(
                &[Serial::Command(format!("~M23 0:/user/{name}"))],
                Duration::ZERO,
            )
            .await?;
        }
        Ok(Uploaded {
            path: format!("0:/user/{name}"),
            print_started: start,
        })
    }
}

/// `upload_path.parent_path()` / `filename()` (séparateur « / »).
fn split_parent(path: &str) -> (&str, &str) {
    match path.rsplit_once('/') {
        Some((p, n)) => (p, n),
        None => ("", path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filenames_follow_orca() {
        assert_eq!(
            flashforge_filename("dir/My part (2).gcode", ".gcode"),
            "My_part__2_.gcode"
        );
        assert_eq!(flashforge_filename("", ".gcode"), "print.gcode");
        assert_eq!(creality_safe_filename("a b c.gcode"), "a_b_c.gcode");
        assert_eq!(creality_model_name(Some("F008")), "K2 Plus");
        assert_eq!(creality_model_name(Some("X1")), "unknown (X1)");
        assert_eq!(creality_model_name(None), "unreachable");
        assert_eq!(split_parent("a/b/c.gcode"), ("a/b", "c.gcode"));
    }

    #[test]
    fn flashforge_api_errors_are_reported() {
        assert!(flashforge_validate(r#"{"code":0}"#).is_ok());
        let e = flashforge_validate(r#"{"code":"3","msg":"bad code"}"#).unwrap_err();
        assert_eq!(e.to_string(), "Flashforge local API error 3: bad code");
        assert!(flashforge_validate("[]").is_err());
        assert!(flashforge_validate(r#"{"err":1}"#)
            .unwrap_err()
            .to_string()
            .contains("Request failed"));
    }
}
