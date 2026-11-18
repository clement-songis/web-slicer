//! ElegooLink (Utils/ElegooLink.cpp d'Orca 2.4.1), trois protocoles selon
//! `printer_model` (classify_printer_model) :
//! - autres modèles : protocole OctoPrint (ElegooLink hérite d'OctoPrint ;
//!   validate_version_text accepte toute version) ;
//! - Centauri Carbon (« CC ») : test `GET /` (« ELEGOO » dans la page), envoi
//!   par morceaux de 1 Mio `POST uploadFile/upload` (Check, S-File-MD5 en hex
//!   majuscule, Offset, Uuid, TotalSize, File), démarrage SDCP par WebSocket
//!   `ws://hôte:3030/websocket` (état Cmd 0 jusqu'à ce que CurrentStatus ne
//!   contienne plus 8, puis Cmd 128, acquittement Ack) ;
//! - Centauri Carbon 2 (« CC2 ») : jeton (clé, défaut 123456), test
//!   `GET system/info?X-Token=` (error_code 0 + system_info.sn), envoi par
//!   morceaux `PUT upload` (Content-Range, X-File-Name, X-File-MD5 minuscule,
//!   X-Token) ; pas de démarrage (post-action None).

use std::time::Duration;

use md5::{Digest, Md5};
use serde_json::{json, Value};

use super::{file_part, json, url_encode, HostClient, HostError, HostInfo, HostResult, Uploaded};

/// MAX_UPLOAD_PACKAGE_LENGTH.
pub const ELEGOO_CHUNK: usize = 1024 * 1024;
/// Port SDCP.
pub const ELEGOO_WS_PORT: u16 = 3030;
const CC2_DEFAULT_TOKEN: &str = "123456";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElegooType {
    Other,
    Cc,
    Cc2,
}

/// classify_printer_model.
pub fn classify(printer_model: Option<&str>) -> ElegooType {
    let m = printer_model.unwrap_or_default();
    if !m.starts_with("Elegoo Centauri") {
        return ElegooType::Other;
    }
    if m.trim_end().ends_with('2') {
        ElegooType::Cc2
    } else {
        ElegooType::Cc
    }
}

/// Message d'erreur d'acquittement SDCP (ElegooLinkStartPrintAck).
pub fn ack_message(ack: i64) -> String {
    let msg = match ack {
        1 => "The printer is busy, Please check the device page for the file and try to start printing again.",
        2 => "The file is lost, please check and try again.",
        3 => "The file is corrupted, please check and try again.",
        4..=6 => "Transmission abnormality, please check and try again.",
        7 => "The file does not match the printer, please check and try again.",
        _ => "Unknown error",
    };
    format!("{msg} Error code: {ack}")
}

/// parse_cc2_response : error_code 0 ; `want_sn` → system_info.sn requis.
fn parse_cc2(body: &str, want_sn: bool) -> HostResult<Option<String>> {
    let v: Value = serde_json::from_str(body)
        .map_err(|_| HostError::Protocol("Error parsing response".into()))?;
    let code = v.get("error_code").and_then(Value::as_i64).unwrap_or(-1);
    if code != 0 {
        let msg = v
            .get("message")
            .and_then(Value::as_str)
            .filter(|m| !m.is_empty())
            .unwrap_or("Printer returned an error");
        return Err(HostError::Protocol(format!("{msg} ({code})")));
    }
    if !want_sn {
        return Ok(None);
    }
    let info = v
        .get("system_info")
        .ok_or_else(|| HostError::Protocol("Missing system_info in response".into()))?;
    let sn = info
        .get("sn")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| HostError::Protocol("Missing printer serial number in response".into()))?;
    Ok(Some(sn.to_string()))
}

fn md5_hex(data: &[u8]) -> String {
    Md5::digest(data)
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect()
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Commande SDCP (format du JSON d'Orca).
fn sdcp(cmd: i64, data: Value) -> Value {
    json!({
        "Id": "",
        "Data": {
            "Cmd": cmd,
            "Data": data,
            "RequestID": uuid::Uuid::new_v4().to_string(),
            "MainboardID": "",
            "TimeStamp": now_ms() as u64,
            "From": 1
        }
    })
}

impl HostClient {
    fn elegoo_type(&self) -> ElegooType {
        classify(self.cfg.printer_model.as_deref())
    }

    fn cc2_token(&self) -> String {
        self.cfg
            .api_key
            .clone()
            .filter(|k| !k.is_empty())
            .unwrap_or_else(|| CC2_DEFAULT_TOKEN.into())
    }

    /// ElegooLink::test.
    pub(super) async fn elegoo_test(&self) -> HostResult<HostInfo> {
        let name = "ElegooLink".to_string();
        match self.elegoo_type() {
            ElegooType::Other => {
                let (_, body) =
                    Self::send(self.with_key(self.http.get(self.url("api/version")))).await?;
                let version = json(&body)?
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                Ok(HostInfo { name, version })
            }
            ElegooType::Cc => {
                let (_, body) = Self::send(self.with_key(self.http.get(self.url("")))).await?;
                if !body.to_ascii_lowercase().contains("elegoo") {
                    return Err(HostError::Protocol("ElegooLink not detected".into()));
                }
                Ok(HostInfo {
                    name,
                    version: None,
                })
            }
            ElegooType::Cc2 => {
                let token = self.cc2_token();
                let resp = self
                    .http
                    .get(self.url(&format!("system/info?X-Token={}", url_encode(&token))))
                    .header("X-Token", &token)
                    .header("Accept", "application/json")
                    .send()
                    .await
                    .map_err(|e| HostError::Network(e.to_string()))?;
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                if status == 401 || status == 403 {
                    return Err(HostError::Status(status, "Invalid access code".into()));
                }
                if !(200..300).contains(&status) {
                    return Err(HostError::Status(status, body.chars().take(300).collect()));
                }
                let sn = parse_cc2(&body, true)?;
                Ok(HostInfo { name, version: sn })
            }
        }
    }

    /// ElegooLink::upload.
    pub(super) async fn elegoo_upload(
        &self,
        filename: &str,
        data: Vec<u8>,
        start: bool,
    ) -> HostResult<Uploaded> {
        let base = filename.rsplit('/').next().unwrap_or(filename).to_string();
        match self.elegoo_type() {
            ElegooType::Other => {
                self.elegoo_test().await?;
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
            ElegooType::Cc => {
                self.elegoo_test().await?;
                let md5 = md5_hex(&data);
                let uuid = uuid::Uuid::new_v4().to_string();
                let total = data.len();
                for (i, chunk) in data.chunks(ELEGOO_CHUNK).enumerate() {
                    let form = reqwest::multipart::Form::new()
                        .text("Check", "1")
                        .text("S-File-MD5", md5.clone())
                        .text("Offset", (i * ELEGOO_CHUNK).to_string())
                        .text("Uuid", uuid.clone())
                        .text("TotalSize", total.to_string())
                        .part(
                            "File",
                            reqwest::multipart::Part::bytes(chunk.to_vec()).file_name(base.clone()),
                        );
                    let (_, body) = Self::send(
                        self.with_key(self.http.post(self.url("uploadFile/upload")))
                            .multipart(form),
                    )
                    .await?;
                    let v: Value = serde_json::from_str(&body)
                        .map_err(|_| HostError::Protocol("Error parsing response".into()))?;
                    let code = v.get("code").and_then(Value::as_str).unwrap_or_default();
                    if code != "000000" {
                        let mut msg = format!("Error code: {code}\n");
                        for m in v
                            .get("messages")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                        {
                            msg += &format!(
                                "{}:{}\n",
                                m.get("field").and_then(Value::as_str).unwrap_or_default(),
                                m.get("message").and_then(Value::as_str).unwrap_or_default()
                            );
                        }
                        return Err(HostError::Protocol(msg));
                    }
                }
                if start {
                    self.elegoo_sdcp_start(&base).await?;
                }
                Ok(Uploaded {
                    path: format!("/local/{base}"),
                    print_started: start,
                })
            }
            ElegooType::Cc2 => {
                self.elegoo_test().await?;
                let token = self.cc2_token();
                let md5 = md5_hex(&data).to_ascii_lowercase();
                let total = data.len();
                if total == 0 {
                    return Err(HostError::Protocol(
                        "The file is empty or could not be read.".into(),
                    ));
                }
                for (i, chunk) in data.chunks(ELEGOO_CHUNK).enumerate() {
                    let offset = i * ELEGOO_CHUNK;
                    let resp = self
                        .http
                        .put(self.url("upload"))
                        .header("Accept", "application/json")
                        .header("Content-Type", "application/octet-stream")
                        .header("Content-Length", chunk.len().to_string())
                        .header(
                            "Content-Range",
                            format!("bytes {offset}-{}/{total}", offset + chunk.len() - 1),
                        )
                        .header("X-File-Name", &base)
                        .header("X-File-MD5", &md5)
                        .header("X-Token", &token)
                        .body(chunk.to_vec())
                        .send()
                        .await
                        .map_err(|e| HostError::Network(e.to_string()))?;
                    let status = resp.status().as_u16();
                    let body = resp.text().await.unwrap_or_default();
                    if status == 401 || status == 403 {
                        return Err(HostError::Status(status, "Invalid access code".into()));
                    }
                    if !(200..300).contains(&status) {
                        return Err(HostError::Status(status, body.chars().take(300).collect()));
                    }
                    parse_cc2(&body, false)?;
                }
                // CC2 : post-action None (Orca ne démarre pas l'impression).
                Ok(Uploaded {
                    path: base,
                    print_started: false,
                })
            }
        }
    }

    /// Démarrage SDCP (checkResult puis print, ElegooLink.cpp:1090-1290).
    async fn elegoo_sdcp_start(&self, filename: &str) -> HostResult<()> {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::Message;
        let port = self.cfg.elegoo_ws_port.unwrap_or(ELEGOO_WS_PORT);
        let url = format!("ws://{}:{port}/websocket", self.host_name());
        let (mut ws, _) = tokio::time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async(&url),
        )
        .await
        .map_err(|_| HostError::Network(format!("{url} : délai dépassé")))?
        .map_err(|e| HostError::Network(format!(
            "The file has been transferred, but some unknown errors occurred. Please check the device page for the file and try to start printing again. ({e})"
        )))?;
        let step = self.cfg.ws_delay;
        let send = |v: Value| Message::Text(v.to_string().into());
        let net = |e: tokio_tungstenite::tungstenite::Error| {
            HostError::Network(format!("Start print failed\n{e}"))
        };
        tokio::time::sleep(step).await;
        // checkResult : attendre que l'état ne contienne plus 8 (occupé), 60 s max.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        let mut need_write = true;
        loop {
            if need_write {
                ws.send(send(sdcp(0, json!({})))).await.map_err(net)?;
                need_write = false;
            }
            let msg = tokio::time::timeout_at(deadline, ws.next())
                .await
                .map_err(|_| HostError::Protocol("Start print timeout".into()))?
                .ok_or_else(|| HostError::Network("Start print failed".into()))?
                .map_err(net)?;
            let Message::Text(text) = msg else { continue };
            let v: Value = serde_json::from_str(&text)
                .map_err(|e| HostError::Protocol(format!("Start print failed\n{e}")))?;
            if let Some(status) = v.get("Status") {
                match status.get("CurrentStatus").and_then(Value::as_array) {
                    Some(cur) if cur.iter().any(|s| s.as_i64() == Some(8)) => {
                        need_write = true;
                        tokio::time::sleep(step).await;
                    }
                    _ => break,
                }
            }
        }
        tokio::time::sleep(step).await;
        // print : Cmd 128, réglages par défaut du dialogue d'envoi (0).
        ws.send(send(sdcp(
            128,
            json!({
                "Filename": format!("/local/{filename}"),
                "StartLayer": 0,
                "Calibration_switch": 0,
                "PrintPlatformType": 0,
                "Tlp_Switch": 0
            }),
        )))
        .await
        .map_err(net)?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            let msg = tokio::time::timeout_at(deadline, ws.next())
                .await
                .map_err(|_| HostError::Protocol("Start print timeout".into()))?
                .ok_or_else(|| HostError::Network("Start print failed".into()))?
                .map_err(net)?;
            let Message::Text(text) = msg else { continue };
            let Ok(v) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let Some(data) = v.get("Data") else { continue };
            if data.get("Cmd").and_then(Value::as_i64) != Some(128) {
                continue;
            }
            let ack = data
                .get("Data")
                .and_then(|d| d.get("Ack"))
                .and_then(Value::as_i64)
                .ok_or_else(|| HostError::Protocol("Error code not found".into()))?;
            let _ = ws.close(None).await;
            return if ack == 0 {
                Ok(())
            } else {
                Err(HostError::Protocol(ack_message(ack)))
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_and_helpers_follow_orca() {
        assert_eq!(classify(Some("Elegoo Centauri Carbon")), ElegooType::Cc);
        assert_eq!(classify(Some("Elegoo Centauri Carbon 2 ")), ElegooType::Cc2);
        assert_eq!(classify(Some("Elegoo Neptune 4")), ElegooType::Other);
        assert_eq!(classify(None), ElegooType::Other);
        assert_eq!(md5_hex(b"abc"), "900150983CD24FB0D6963F7D28E17F72");
        assert!(ack_message(1).starts_with("The printer is busy"));
        assert!(ack_message(9).ends_with("Error code: 9"));
        assert_eq!(
            parse_cc2(r#"{"error_code":0,"system_info":{"sn":"X1"}}"#, true)
                .unwrap()
                .as_deref(),
            Some("X1")
        );
        assert!(parse_cc2(r#"{"error_code":5,"message":"bad"}"#, false)
            .unwrap_err()
            .to_string()
            .contains("bad (5)"));
        assert!(parse_cc2(r#"{"error_code":0}"#, true).is_err());
    }
}
