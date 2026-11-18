//! Recherche réseau propre à deux constructeurs (bouton « Browse » de
//! `PhysicalPrinterDialog.cpp:218-253` d'Orca 2.4.1), hors DNS-SD à nom fixe :
//!
//! - **Creality** (`Utils/CrealityHostDiscovery.cpp`, `deps_src/mdns/cxmdns.cpp`) :
//!   les K-series s'annoncent sous un type de service unique par appareil
//!   (`_Creality-<MAC>._udp.local`). Orca envoie la méta-requête DNS-SD
//!   `_services._dns-sd._udp.local` (PTR, bit QU) en multicast depuis un port
//!   éphémère, garde les réponses (id 0, drapeaux 0x8400) dont la cible PTR
//!   contient « Creality »/« creality », dédoublonne par IP puis interroge
//!   `http://<ip>/info` (connexion 2 s, total 4 s) pour le modèle et la MAC.
//! - **Flashforge** (`Utils/Flashforge.cpp`, `discover_printers`) : datagramme
//!   de 20 octets diffusé vers le port UDP 48899 (255.255.255.255,
//!   192.168.0.255, 192.168.1.255 et le /24 de chaque adresse de l'hôte),
//!   écoute sur le port 18007 ; réponse ≥ 0xC4 octets : nom (32 octets à 0),
//!   n° de série (32 octets à 0x92). 10 s par essai, arrêt 1,5 s après la
//!   dernière réponse, 3 essais tant que rien n'est trouvé.
//!
//! Comme pour `bonjour`, c'est le serveur web-slicer (sur le réseau local de
//! l'imprimante) qui cherche. Les résultats reprennent la forme `BonjourReply`
//! pour que le formulaire d'imprimante les affiche comme les autres.

use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use super::bonjour::{parse_records, BonjourReply, Record, MDNS_ADDR};

// — Creality ———————————————————————————————————————————————————————————

/// Méta-requête DNS-SD d'Orca (`mdns_services_query`, mdns.h:882) : id 0,
/// une question PTR `_services._dns-sd._udp.local`, classe IN + bit QU.
pub const SERVICES_QUERY: &[u8] = &[
    0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
    0x09, b'_', b's', b'e', b'r', b'v', b'i', b'c', b'e', b's', //
    0x07, b'_', b'd', b'n', b's', b'-', b's', b'd', //
    0x04, b'_', b'u', b'd', b'p', 0x05, b'l', b'o', b'c', b'a', b'l', 0x00, //
    0x00, 0x0C, 0x80, 0x01,
];

const SERVICES_DOMAIN: &str = "_services._dns-sd._udp.local";
const CREALITY_PREFIXES: [&str; 2] = ["Creality", "creality"];

/// Modèles à CFS reconnus par Orca (`kCfsCapableModels`).
const CFS_MODELS: [(&str, &str); 3] = [("F008", "K2 Plus"), ("F012", "K2 Pro"), ("F021", "K2")];

/// Imprimante Creality trouvée (`CrealityHost`, CrealityHostDiscovery.hpp:10).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CrealityHost {
    pub ip: String,
    pub service_name: String,
    pub hostname: String,
    pub model_code: String,
    pub model_name: String,
    pub mac: String,
    pub cfs_capable: bool,
}

impl CrealityHost {
    /// Colonne « Model » de CrealityDiscoveryDialog::run_discovery.
    pub fn model_label(&self) -> &str {
        if !self.model_name.is_empty() {
            &self.model_name
        } else if self.cfs_capable {
            "(unknown K-series)"
        } else {
            "Creality"
        }
    }

    /// Ligne affichable ; l'adresse retenue est `http://<ip>` (Orca :
    /// `"http://" + dialog.selected_ip()`).
    pub fn into_reply(self) -> BonjourReply {
        let mut txt = BTreeMap::new();
        txt.insert("service".into(), self.service_name.clone());
        if !self.model_code.is_empty() {
            txt.insert("model".into(), self.model_code.clone());
        }
        if !self.mac.is_empty() {
            txt.insert("mac".into(), self.mac.clone());
        }
        BonjourReply {
            full_address: format!("http://{}", self.ip),
            port: 80,
            service_name: self.model_label().to_string(),
            hostname: self.hostname,
            ip: self.ip,
            txt,
        }
    }
}

/// `hostname_from_service` : « K2- » + 4 derniers caractères du suffixe
/// (`_Creality-543324280CDB19._udp.local.` → `K2-DB19`).
pub fn hostname_from_service(service: &str) -> String {
    let Some(dash) = service.rfind('-') else {
        return String::new();
    };
    let Some(dot) = service[dash..].find('.').map(|d| dash + d) else {
        return String::new();
    };
    let suffix = &service[dash + 1..dot];
    if suffix.is_empty() {
        return String::new();
    }
    let tail = suffix
        .char_indices()
        .rev()
        .nth(3)
        .map_or(suffix, |(i, _)| &suffix[i..]);
    format!("K2-{tail}")
}

/// Réponses Creality d'un paquet (règles de `recvMachineInfoFromSocket`) :
/// cibles PTR de `_services._dns-sd._udp.local` contenant le préfixe.
pub fn creality_services(packet: &[u8]) -> Vec<String> {
    // RFC 6762 : id 0 ; Orca exige exactement les drapeaux 0x8400.
    if packet.len() < 12 || packet[0..2] != [0, 0] || packet[2..4] != [0x84, 0x00] {
        return Vec::new();
    }
    parse_records(packet)
        .into_iter()
        .filter_map(|r| match r {
            Record::Ptr { name, target } if name == SERVICES_DOMAIN => Some(target),
            _ => None,
        })
        .filter(|t| CREALITY_PREFIXES.iter().any(|p| t.contains(p)))
        .collect()
}

/// Lecture de `/info` (`probe_info`) : modèle, MAC, famille K2.
pub fn apply_info(host: &mut CrealityHost, body: &str) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return;
    };
    if let Some(m) = v.get("model").and_then(|m| m.as_str()) {
        host.model_code = m.to_string();
    }
    if let Some(m) = v.get("mac").and_then(|m| m.as_str()) {
        host.mac = m.to_string();
    }
    if let Some((_, name)) = CFS_MODELS.iter().find(|(c, _)| *c == host.model_code) {
        host.cfs_capable = true;
        host.model_name = (*name).to_string();
    }
}

async fn probe_info(host: &mut CrealityHost, port: u16) {
    let Ok(client) = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(4))
        .build()
    else {
        return;
    };
    let url = if port == 80 {
        format!("http://{}/info", host.ip)
    } else {
        format!("http://{}:{port}/info", host.ip)
    };
    match client.get(&url).send().await {
        Ok(res) => {
            if let Ok(body) = res.text().await {
                apply_info(host, &body);
            }
        }
        Err(e) => tracing::info!(ip = %host.ip, error = %e, "Creality : /info injoignable"),
    }
}

/// Recherche Creality vers `target` (normalement [`MDNS_ADDR`]). Orca écoute
/// tant que des réponses arrivent à moins de 5 s d'intervalle ; ici la durée
/// totale est bornée par `timeout` (requête HTTP). `info_port` : 80 sauf tests.
pub async fn discover_creality_at(
    target: SocketAddr,
    timeout: Duration,
    info_port: u16,
) -> std::io::Result<Vec<CrealityHost>> {
    let socket = tokio::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
    if let IpAddr::V4(v4) = target.ip() {
        if v4.is_multicast() {
            socket.set_multicast_ttl_v4(1)?;
        }
    }
    socket.send_to(SERVICES_QUERY, target).await?;
    let deadline = tokio::time::Instant::now() + timeout;
    let mut hosts: Vec<CrealityHost> = Vec::new();
    let mut buf = vec![0u8; 2048];
    loop {
        match tokio::time::timeout_at(deadline, socket.recv_from(&mut buf)).await {
            Ok(Ok((n, from))) => {
                for service in creality_services(&buf[..n]) {
                    let ip = from.ip().to_string();
                    if hosts.iter().any(|h| h.ip == ip) {
                        continue; // dédoublonnage par IP
                    }
                    hosts.push(CrealityHost {
                        ip,
                        hostname: hostname_from_service(&service),
                        service_name: service,
                        ..Default::default()
                    });
                }
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => break,
        }
    }
    for h in &mut hosts {
        probe_info(h, info_port).await;
    }
    Ok(hosts)
}

/// Recherche Creality sur le réseau local (méta-requête multicast).
pub async fn discover_creality(timeout: Duration) -> std::io::Result<Vec<BonjourReply>> {
    Ok(discover_creality_at(MDNS_ADDR, timeout, 80)
        .await?
        .into_iter()
        .map(CrealityHost::into_reply)
        .collect())
}

// — Flashforge ——————————————————————————————————————————————————————————

pub const FLASHFORGE_DISCOVERY_PORT: u16 = 48899;
pub const FLASHFORGE_LISTEN_PORT: u16 = 18007;
pub const FLASHFORGE_DISCOVERY_MESSAGE: [u8; 20] = [
    0x77, 0x77, 0x77, 0x2e, 0x75, 0x73, 0x72, 0x22, 0x65, 0x36, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
];

/// Imprimante Flashforge trouvée (`FlashforgeDiscoveredPrinter`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashforgePrinter {
    pub name: String,
    pub serial_number: String,
    pub ip_address: String,
}

impl FlashforgePrinter {
    /// Orca renseigne `print_host` = IP et `flashforge_serial_number`.
    pub fn into_reply(self) -> BonjourReply {
        let mut txt = BTreeMap::new();
        txt.insert("serial_number".into(), self.serial_number.clone());
        BonjourReply {
            full_address: self.ip_address.clone(),
            port: 8898,
            service_name: self.name,
            hostname: self.serial_number,
            ip: self.ip_address,
            txt,
        }
    }
}

fn trim_null_terminated(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

/// `parse_discovery_response` : ≥ 0xC4 octets, nom à 0, série à 0x92.
pub fn parse_flashforge_response(packet: &[u8], ip: &str) -> Option<FlashforgePrinter> {
    if packet.len() < 0xC4 {
        return None;
    }
    let name = trim_null_terminated(&packet[..32]);
    let serial_number = trim_null_terminated(&packet[0x92..0x92 + 32]);
    if name.is_empty() && serial_number.is_empty() {
        return None;
    }
    Some(FlashforgePrinter {
        name,
        serial_number,
        ip_address: ip.to_string(),
    })
}

/// Adresses de diffusion (`get_discovery_broadcast_addresses`) : trois fixes
/// + le /24 de chaque adresse IPv4 (hors 127/8) du nom d'hôte résolu.
pub async fn flashforge_broadcast_addresses() -> Vec<Ipv4Addr> {
    let mut out: BTreeSet<Ipv4Addr> = [
        Ipv4Addr::BROADCAST,
        Ipv4Addr::new(192, 168, 0, 255),
        Ipv4Addr::new(192, 168, 1, 255),
    ]
    .into_iter()
    .collect();
    let host = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty());
    if let Some(host) = host {
        if let Ok(addrs) = tokio::net::lookup_host((host.as_str(), 0)).await {
            for a in addrs {
                if let IpAddr::V4(v4) = a.ip() {
                    let o = v4.octets();
                    if o[0] != 127 {
                        out.insert(Ipv4Addr::new(o[0], o[1], o[2], 255));
                    }
                }
            }
        }
    }
    out.into_iter().collect()
}

/// Paramètres de `Flashforge::discover_printers` (défauts d'Orca).
#[derive(Debug, Clone)]
pub struct FlashforgeScan {
    pub targets: Vec<SocketAddr>,
    pub listen_port: u16,
    pub timeout: Duration,
    pub idle_timeout: Duration,
    pub max_retries: u32,
}

/// Recherche Flashforge ; résultat trié par nom puis IP, dédoublonné par IP.
pub async fn discover_flashforge_with(
    scan: &FlashforgeScan,
) -> std::io::Result<Vec<FlashforgePrinter>> {
    let mut by_ip: BTreeMap<String, FlashforgePrinter> = BTreeMap::new();
    for _ in 0..scan.max_retries.max(1) {
        let socket = tokio::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, scan.listen_port)).await?;
        socket.set_broadcast(true)?;
        for t in &scan.targets {
            // Erreurs d'envoi ignorées (réseau absent), comme Orca.
            let _ = socket.send_to(&FLASHFORGE_DISCOVERY_MESSAGE, t).await;
        }
        let start = tokio::time::Instant::now();
        let mut last_reply = start;
        let mut buf = vec![0u8; 512];
        loop {
            let mut wake = start + scan.timeout;
            if !by_ip.is_empty() {
                wake = wake.min(last_reply + scan.idle_timeout);
            }
            match tokio::time::timeout_at(wake, socket.recv_from(&mut buf)).await {
                Ok(Ok((n, from))) => {
                    if let Some(p) = parse_flashforge_response(&buf[..n], &from.ip().to_string()) {
                        by_ip.insert(p.ip_address.clone(), p);
                        last_reply = tokio::time::Instant::now();
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => break,
            }
        }
        if !by_ip.is_empty() {
            break;
        }
    }
    let mut printers: Vec<FlashforgePrinter> = by_ip.into_values().collect();
    printers.sort_by(|a, b| a.name.cmp(&b.name).then(a.ip_address.cmp(&b.ip_address)));
    Ok(printers)
}

/// Recherche Flashforge sur le réseau local ; `timeout` par essai.
pub async fn discover_flashforge(timeout: Duration) -> std::io::Result<Vec<BonjourReply>> {
    let targets = flashforge_broadcast_addresses()
        .await
        .into_iter()
        .map(|a| SocketAddr::from((a, FLASHFORGE_DISCOVERY_PORT)))
        .collect();
    let scan = FlashforgeScan {
        targets,
        listen_port: FLASHFORGE_LISTEN_PORT,
        timeout,
        idle_timeout: Duration::from_millis(1500),
        max_retries: 3,
    };
    Ok(discover_flashforge_with(&scan)
        .await?
        .into_iter()
        .map(FlashforgePrinter::into_reply)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Réponse mDNS d'une K2 : PTR `_services._dns-sd._udp.local` → service.
    fn creality_packet(service: &str, flags: [u8; 2]) -> Vec<u8> {
        let mut p = vec![0, 0, flags[0], flags[1], 0, 0, 0, 1, 0, 0, 0, 0];
        for l in SERVICES_DOMAIN.split('.') {
            p.push(l.len() as u8);
            p.extend_from_slice(l.as_bytes());
        }
        p.push(0);
        p.extend_from_slice(&[0, 12, 0, 1, 0, 0, 0x11, 0x94]);
        let mut rd = Vec::new();
        for l in service.split('.').filter(|l| !l.is_empty()) {
            rd.push(l.len() as u8);
            rd.extend_from_slice(l.as_bytes());
        }
        rd.push(0);
        p.extend_from_slice(&(rd.len() as u16).to_be_bytes());
        p.extend_from_slice(&rd);
        p
    }

    fn flashforge_packet(name: &str, serial: &str) -> Vec<u8> {
        let mut p = vec![0u8; 0xC4];
        p[..name.len()].copy_from_slice(name.as_bytes());
        p[0x92..0x92 + serial.len()].copy_from_slice(serial.as_bytes());
        p
    }

    #[test]
    fn services_query_is_orcas_meta_query() {
        assert_eq!(SERVICES_QUERY.len(), 46);
        assert_eq!(
            &SERVICES_QUERY[SERVICES_QUERY.len() - 4..],
            &[0, 12, 0x80, 1]
        );
    }

    #[test]
    fn creality_hostname_and_filters() {
        assert_eq!(
            hostname_from_service("_Creality-543324280CDB19._udp.local."),
            "K2-DB19"
        );
        assert_eq!(hostname_from_service("_Creality-AB._udp.local"), "K2-AB");
        assert_eq!(hostname_from_service("nodash.local"), "");
        let pkt = creality_packet("_Creality-543324280CDB19._udp.local", [0x84, 0]);
        assert_eq!(
            creality_services(&pkt),
            vec!["_Creality-543324280CDB19._udp.local"]
        );
        // Autres services ou drapeaux ≠ 0x8400 : ignorés (cxmdns.cpp).
        assert!(creality_services(&creality_packet("_octoprint._tcp.local", [0x84, 0])).is_empty());
        assert!(
            creality_services(&creality_packet("_Creality-1._udp.local", [0x80, 0])).is_empty()
        );
        for n in 0..pkt.len() {
            let _ = creality_services(&pkt[..n]);
        }
    }

    #[test]
    fn creality_info_and_labels() {
        let mut h = CrealityHost {
            ip: "10.0.0.9".into(),
            ..Default::default()
        };
        apply_info(&mut h, r#"{"model":"F012","mac":"54:33"}"#);
        assert_eq!((h.model_name.as_str(), h.cfs_capable), ("K2 Pro", true));
        let r = h.into_reply();
        assert_eq!(r.full_address, "http://10.0.0.9");
        assert_eq!(r.service_name, "K2 Pro");
        let mut other = CrealityHost::default();
        apply_info(&mut other, r#"{"model":"K1C"}"#);
        assert_eq!(other.model_label(), "Creality");
        apply_info(&mut other, "pas du json");
        assert_eq!(other.model_code, "K1C");
    }

    #[test]
    fn flashforge_response_parsing() {
        let pkt = flashforge_packet("Adventurer 5M", "SNMOMC9900728");
        let p = parse_flashforge_response(&pkt, "1.2.3.4").unwrap();
        assert_eq!(p.name, "Adventurer 5M");
        assert_eq!(p.serial_number, "SNMOMC9900728");
        assert!(parse_flashforge_response(&[0u8; 0xC3], "x").is_none());
        assert!(parse_flashforge_response(&[0u8; 0xC4], "x").is_none());
    }

    #[tokio::test]
    async fn creality_discovery_with_fake_responder_and_info() {
        // /info factice (HTTP minimal).
        let http = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let info_port = http.local_addr().unwrap().port();
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut s, _) = http.accept().await.unwrap();
            let mut req = [0u8; 1024];
            let n = s.read(&mut req).await.unwrap();
            assert!(String::from_utf8_lossy(&req[..n]).starts_with("GET /info "));
            let body = r#"{"model":"F008","mac":"543324280CDB"}"#;
            let res = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            s.write_all(res.as_bytes()).await.unwrap();
        });
        let responder = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = responder.local_addr().unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 512];
            let (n, from) = responder.recv_from(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], SERVICES_QUERY);
            let pkt = creality_packet("_Creality-543324280CDB19._udp.local", [0x84, 0]);
            // Deux annonces (multi-interface) + un service étranger.
            responder.send_to(&pkt, from).await.unwrap();
            responder.send_to(&pkt, from).await.unwrap();
            let other = creality_packet("_http._tcp.local", [0x84, 0]);
            responder.send_to(&other, from).await.unwrap();
        });
        let hosts = discover_creality_at(addr, Duration::from_millis(400), info_port)
            .await
            .unwrap();
        assert_eq!(hosts.len(), 1, "{hosts:?}");
        let r = hosts[0].clone().into_reply();
        assert_eq!(r.full_address, "http://127.0.0.1");
        assert_eq!(r.hostname, "K2-DB19");
        assert_eq!(r.service_name, "K2 Plus");
        assert_eq!(r.txt["mac"], "543324280CDB");
    }

    #[tokio::test]
    async fn flashforge_discovery_with_fake_responder() {
        let responder = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = responder.local_addr().unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 512];
            let (n, from) = responder.recv_from(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &FLASHFORGE_DISCOVERY_MESSAGE);
            responder.send_to(&[1, 2, 3], from).await.unwrap(); // trop court : ignoré
            let pkt = flashforge_packet("AD5M", "SN123");
            responder.send_to(&pkt, from).await.unwrap();
        });
        // Port d'écoute libre (18007 en production).
        let listen_port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let scan = FlashforgeScan {
            targets: vec![addr],
            listen_port,
            timeout: Duration::from_secs(3),
            idle_timeout: Duration::from_millis(200),
            max_retries: 3,
        };
        let t0 = std::time::Instant::now();
        let found = discover_flashforge_with(&scan).await.unwrap();
        assert!(
            t0.elapsed() < Duration::from_secs(2),
            "arrêt après inactivité"
        );
        assert_eq!(found.len(), 1, "{found:?}");
        let r = found[0].clone().into_reply();
        assert_eq!(r.full_address, "127.0.0.1");
        assert_eq!(r.service_name, "AD5M");
        assert_eq!(r.txt["serial_number"], "SN123");
    }

    #[tokio::test]
    async fn flashforge_retries_then_returns_empty() {
        let silent = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let scan = FlashforgeScan {
            targets: vec![silent.local_addr().unwrap()],
            listen_port: 0,
            timeout: Duration::from_millis(100),
            idle_timeout: Duration::from_millis(50),
            max_retries: 3,
        };
        assert!(discover_flashforge_with(&scan).await.unwrap().is_empty());
        let mut buf = [0u8; 64];
        for _ in 0..3 {
            let (n, _) = silent.recv_from(&mut buf).await.unwrap();
            assert_eq!(n, 20, "un envoi par essai");
        }
    }
}
