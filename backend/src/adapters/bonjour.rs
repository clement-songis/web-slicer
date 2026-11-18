//! Découverte mDNS / DNS-SD des hôtes d'impression (Orca : `Utils/Bonjour.cpp`,
//! bouton « Browse » du dialogue d'imprimante physique, `BonjourDialog.cpp` qui
//! cherche le service `octoprint`).
//!
//! Le serveur web-slicer est sur le réseau local de l'imprimante : c'est lui qui
//! interroge, pas le navigateur. Requête PTR `_<service>._tcp.local` envoyée en
//! multicast (224.0.0.251:5353) depuis un port éphémère : les répondeurs mDNS
//! répondent alors en unicast à ce port (RFC 6762 §6.7, « legacy unicast »),
//! sans avoir à rejoindre le groupe. Les enregistrements PTR → SRV → A + TXT
//! sont assemblés comme `BonjourReply` d'Orca (adresse complète, `path` TXT).

use std::collections::{BTreeMap, HashMap};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use serde::Serialize;
use ts_rs::TS;

/// Groupe mDNS IPv4.
pub const MDNS_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(224, 0, 0, 251)), 5353);

const TYPE_A: u16 = 1;
const TYPE_PTR: u16 = 12;
const TYPE_TXT: u16 = 16;
const TYPE_SRV: u16 = 33;

/// Hôte trouvé (champs de `BonjourReply`, Bonjour.hpp:21).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct BonjourReply {
    pub ip: String,
    pub port: u16,
    /// Nom d'instance du service (ex. « OctoPrint instance on pi »).
    pub service_name: String,
    pub hostname: String,
    /// `[https://]ip[:port][path]` (règles de BonjourReply::BonjourReply).
    pub full_address: String,
    #[ts(type = "Record<string, string>")]
    pub txt: BTreeMap<String, String>,
}

impl BonjourReply {
    /// Construit la réponse comme Orca (Bonjour.cpp:1050) : `https://` pour le
    /// port 443, port omis pour 80/443, `path` TXT (défaut `/`) ajouté s'il
    /// n'est pas `/`.
    pub fn new(
        ip: IpAddr,
        port: u16,
        service_name: String,
        hostname: String,
        mut txt: BTreeMap<String, String>,
    ) -> Self {
        let proto = if port == 443 { "https://" } else { "" };
        let port_suffix = if port != 443 && port != 80 {
            format!(":{port}")
        } else {
            String::new()
        };
        let mut path = txt.get("path").cloned().unwrap_or_else(|| "/".into());
        if !path.starts_with('/') {
            path.insert(0, '/');
        }
        let mut full_address = format!("{proto}{ip}{port_suffix}");
        if path != "/" {
            full_address.push_str(&path);
        }
        txt.insert("path".into(), path);
        Self {
            ip: ip.to_string(),
            port,
            service_name,
            hostname,
            full_address,
            txt,
        }
    }
}

/// Service valide : étiquette DNS simple (`octoprint`, `moonraker`, `http`…).
pub fn valid_service(service: &str) -> bool {
    !service.is_empty()
        && service.len() <= 63
        && service
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn service_domain(service: &str) -> String {
    format!("_{service}._tcp.local")
}

fn push_name(out: &mut Vec<u8>, name: &str) {
    for label in name.split('.').filter(|l| !l.is_empty()) {
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
}

/// Requête DNS-SD : une question PTR `_<service>._tcp.local`, classe IN.
pub fn build_query(service: &str, id: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(&[0, 0]); // flags : requête standard
    out.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0]); // 1 question
    push_name(&mut out, &service_domain(service));
    out.extend_from_slice(&TYPE_PTR.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out
}

/// Enregistrement de ressource décodé (types utiles seulement).
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    Ptr {
        name: String,
        target: String,
    },
    Srv {
        name: String,
        port: u16,
        target: String,
    },
    Txt {
        name: String,
        entries: BTreeMap<String, String>,
    },
    A {
        name: String,
        ip: Ipv4Addr,
    },
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn u16(&mut self) -> Option<u16> {
        let b = self.buf.get(self.pos..self.pos + 2)?;
        self.pos += 2;
        Some(u16::from_be_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Option<u32> {
        let b = self.buf.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    /// Nom DNS avec compression (pointeurs bornés pour éviter les boucles).
    fn name(&mut self) -> Option<String> {
        let mut labels = Vec::new();
        let mut pos = self.pos;
        let mut jumped = false;
        for _ in 0..128 {
            let len = *self.buf.get(pos)?;
            if len & 0xC0 == 0xC0 {
                let lo = *self.buf.get(pos + 1)?;
                if !jumped {
                    self.pos = pos + 2;
                }
                jumped = true;
                pos = (usize::from(len & 0x3F) << 8) | usize::from(lo);
                continue;
            }
            if len == 0 {
                if !jumped {
                    self.pos = pos + 1;
                }
                return Some(labels.join("."));
            }
            let label = self.buf.get(pos + 1..pos + 1 + usize::from(len))?;
            labels.push(String::from_utf8_lossy(label).into_owned());
            pos += 1 + usize::from(len);
        }
        None
    }
}

/// Décode les enregistrements (réponses, autorité, additionnels) d'un paquet.
/// Paquet tronqué ou malformé : enregistrements lus jusque-là.
pub fn parse_records(buf: &[u8]) -> Vec<Record> {
    let mut out = Vec::new();
    let mut r = Reader { buf, pos: 0 };
    let header = (|| {
        let _id = r.u16()?;
        let flags = r.u16()?;
        let qd = r.u16()?;
        let an = r.u16()?;
        let ns = r.u16()?;
        let ar = r.u16()?;
        Some((flags, qd, u32::from(an) + u32::from(ns) + u32::from(ar)))
    })();
    let Some((flags, qd, rr)) = header else {
        return out;
    };
    if flags & 0x8000 == 0 {
        return out; // pas une réponse
    }
    for _ in 0..qd {
        if r.name().is_none() || r.u32().is_none() {
            return out;
        }
    }
    for _ in 0..rr {
        let Some(name) = r.name() else { break };
        let (Some(rtype), Some(_class), Some(_ttl), Some(len)) =
            (r.u16(), r.u16(), r.u32(), r.u16())
        else {
            break;
        };
        let start = r.pos;
        let end = start + usize::from(len);
        if end > buf.len() {
            break;
        }
        let name_l = name.to_ascii_lowercase();
        match rtype {
            TYPE_PTR => {
                if let Some(target) = r.name() {
                    out.push(Record::Ptr {
                        name: name_l,
                        target,
                    });
                }
            }
            TYPE_SRV => {
                let _prio = r.u16();
                let _weight = r.u16();
                if let (Some(port), Some(target)) = (r.u16(), r.name()) {
                    out.push(Record::Srv {
                        name: name_l,
                        port,
                        target,
                    });
                }
            }
            TYPE_TXT => {
                let mut entries = BTreeMap::new();
                let mut p = start;
                while p < end {
                    let l = usize::from(buf[p]);
                    let Some(item) = buf.get(p + 1..(p + 1 + l).min(end)) else {
                        break;
                    };
                    let item = String::from_utf8_lossy(item);
                    if let Some((k, v)) = item.split_once('=') {
                        entries.insert(k.to_ascii_lowercase(), v.to_string());
                    } else if !item.is_empty() {
                        entries.insert(item.to_ascii_lowercase(), String::new());
                    }
                    p += 1 + l;
                }
                out.push(Record::Txt {
                    name: name_l,
                    entries,
                });
            }
            TYPE_A if len == 4 => {
                out.push(Record::A {
                    name: name_l,
                    ip: Ipv4Addr::new(buf[start], buf[start + 1], buf[start + 2], buf[start + 3]),
                });
            }
            _ => {}
        }
        r.pos = end;
    }
    out
}

/// Assemble les réponses du service à partir des enregistrements accumulés.
/// `sources` : adresse d'émission par instance, repli si aucun A n'est reçu.
pub fn assemble(
    service: &str,
    records: &[Record],
    sources: &HashMap<String, IpAddr>,
) -> Vec<BonjourReply> {
    let domain = service_domain(service);
    let suffix = format!(".{domain}");
    let mut out: Vec<BonjourReply> = Vec::new();
    for rec in records {
        let Record::Ptr { name, target } = rec else {
            continue;
        };
        if *name != domain {
            continue;
        }
        let instance = target.to_ascii_lowercase();
        let Some((port, host)) = records.iter().find_map(|r| match r {
            Record::Srv { name, port, target } if *name == instance => {
                Some((*port, target.clone()))
            }
            _ => None,
        }) else {
            continue;
        };
        let host_l = host.to_ascii_lowercase();
        let ip = records
            .iter()
            .find_map(|r| match r {
                Record::A { name, ip } if *name == host_l => Some(IpAddr::V4(*ip)),
                _ => None,
            })
            .or_else(|| sources.get(&instance).copied());
        let Some(ip) = ip else { continue };
        let txt = records
            .iter()
            .find_map(|r| match r {
                Record::Txt { name, entries } if *name == instance => Some(entries.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let service_name = target
            .strip_suffix(&suffix)
            .or_else(|| {
                let l = target.len().saturating_sub(suffix.len());
                target
                    .get(l..)
                    .filter(|s| s.eq_ignore_ascii_case(&suffix))
                    .map(|_| &target[..l])
            })
            .unwrap_or(target)
            .to_string();
        let hostname = host.trim_end_matches(".local").to_string();
        let reply = BonjourReply::new(ip, port, service_name, hostname, txt);
        if !out
            .iter()
            .any(|o| o.full_address == reply.full_address && o.service_name == reply.service_name)
        {
            out.push(reply);
        }
    }
    out.sort_by(|a, b| {
        a.service_name
            .cmp(&b.service_name)
            .then(a.full_address.cmp(&b.full_address))
    });
    out
}

/// Interroge `target` (normalement [`MDNS_ADDR`]) pendant `timeout`, la
/// requête étant répétée `retries` fois (Orca : 3 essais, BonjourDialog.cpp:128).
pub async fn discover_at(
    target: SocketAddr,
    service: &str,
    timeout: Duration,
    retries: u32,
) -> std::io::Result<Vec<BonjourReply>> {
    let bind: SocketAddr = if target.is_ipv4() {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    } else {
        (std::net::Ipv6Addr::UNSPECIFIED, 0).into()
    };
    let socket = tokio::net::UdpSocket::bind(bind).await?;
    if let IpAddr::V4(v4) = target.ip() {
        if v4.is_multicast() {
            socket.set_multicast_ttl_v4(255)?;
        }
    }
    let query = build_query(service, 0);
    let retries = retries.max(1);
    let step = timeout / retries;
    let deadline = tokio::time::Instant::now() + timeout;
    let mut next_send = tokio::time::Instant::now();
    let mut records = Vec::new();
    let mut sources: HashMap<String, IpAddr> = HashMap::new();
    let mut sent = 0;
    let mut buf = vec![0u8; 9000];
    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        if sent < retries && now >= next_send {
            socket.send_to(&query, target).await?;
            sent += 1;
            next_send = now + step;
        }
        let wake = if sent < retries {
            next_send.min(deadline)
        } else {
            deadline
        };
        match tokio::time::timeout_at(wake, socket.recv_from(&mut buf)).await {
            Ok(Ok((n, from))) => {
                let recs = parse_records(&buf[..n]);
                for r in &recs {
                    if let Record::Srv { name, .. } | Record::Txt { name, .. } = r {
                        sources.entry(name.clone()).or_insert(from.ip());
                    }
                }
                records.extend(recs);
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {}
        }
    }
    Ok(assemble(service, &records, &sources))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Réponse mDNS forgée : PTR + SRV + TXT + A, avec compression de noms.
    pub(crate) fn sample_response(instance: &str, port: u16, ip: [u8; 4], path: &str) -> Vec<u8> {
        let mut p = vec![0, 0, 0x84, 0, 0, 0, 0, 1, 0, 0, 0, 3];
        // Réponse : PTR _octoprint._tcp.local → <instance>._octoprint._tcp.local
        let svc_off = p.len();
        push_name(&mut p, "_octoprint._tcp.local");
        p.extend_from_slice(&[0, 12, 0, 1, 0, 0, 0x11, 0x94]);
        let rd = vec![instance.len() as u8]
            .into_iter()
            .chain(instance.bytes())
            .chain([0xC0, svc_off as u8])
            .collect::<Vec<_>>();
        p.extend_from_slice(&(rd.len() as u16).to_be_bytes());
        let inst_off = p.len();
        p.extend_from_slice(&rd);
        // SRV <instance> → port, pi.local
        p.extend_from_slice(&[0xC0, inst_off as u8, 0, 33, 0x80, 1, 0, 0, 0, 120]);
        let mut srv = vec![0, 0, 0, 0];
        srv.extend_from_slice(&port.to_be_bytes());
        let host_off = p.len() + 2 + srv.len();
        push_name(&mut srv, "pi.local");
        p.extend_from_slice(&(srv.len() as u16).to_be_bytes());
        p.extend_from_slice(&srv);
        // TXT <instance> : path=…, version=1.9
        p.extend_from_slice(&[0xC0, inst_off as u8, 0, 16, 0x80, 1, 0, 0, 0, 120]);
        let mut txt = Vec::new();
        for e in [format!("path={path}"), "version=1.9".to_string()] {
            txt.push(e.len() as u8);
            txt.extend_from_slice(e.as_bytes());
        }
        p.extend_from_slice(&(txt.len() as u16).to_be_bytes());
        p.extend_from_slice(&txt);
        // A pi.local
        p.extend_from_slice(&[0xC0, host_off as u8, 0, 1, 0x80, 1, 0, 0, 0, 120, 0, 4]);
        p.extend_from_slice(&ip);
        p
    }

    #[test]
    fn query_is_a_ptr_question_for_the_service() {
        let q = build_query("octoprint", 0);
        assert_eq!(&q[..12], &[0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&q[12..23], b"\x0a_octoprint");
        assert_eq!(&q[q.len() - 4..], &[0, 12, 0, 1]);
    }

    #[test]
    fn parses_compressed_response_into_orca_reply() {
        let pkt = sample_response("OctoPrint on pi", 5000, [192, 168, 1, 5], "octo/");
        let recs = parse_records(&pkt);
        assert_eq!(recs.len(), 4, "{recs:?}");
        let replies = assemble("octoprint", &recs, &HashMap::new());
        assert_eq!(replies.len(), 1);
        let r = &replies[0];
        assert_eq!(r.service_name, "OctoPrint on pi");
        assert_eq!(r.hostname, "pi");
        assert_eq!(r.port, 5000);
        assert_eq!(r.ip, "192.168.1.5");
        assert_eq!(r.full_address, "192.168.1.5:5000/octo/");
        assert_eq!(r.txt["version"], "1.9");
        assert_eq!(r.txt["path"], "/octo/");
    }

    #[test]
    fn address_rules_follow_bonjour_reply() {
        let ip: IpAddr = "10.0.0.2".parse().unwrap();
        let r = BonjourReply::new(ip, 443, "s".into(), "h".into(), BTreeMap::new());
        assert_eq!(r.full_address, "https://10.0.0.2");
        let r = BonjourReply::new(ip, 80, "s".into(), "h".into(), BTreeMap::new());
        assert_eq!(r.full_address, "10.0.0.2");
        assert_eq!(r.txt["path"], "/");
    }

    #[test]
    fn malformed_packets_never_panic() {
        let pkt = sample_response("x", 80, [1, 2, 3, 4], "/");
        for n in 0..pkt.len() {
            let _ = parse_records(&pkt[..n]);
        }
        let mut loop_pkt = vec![0, 0, 0x84, 0, 0, 0, 0, 1, 0, 0, 0, 0];
        loop_pkt.extend_from_slice(&[0xC0, 12]); // pointeur sur lui-même
        assert!(parse_records(&loop_pkt).is_empty());
        assert!(!valid_service("Octo"));
        assert!(!valid_service("a.b"));
        assert!(valid_service("octoprint"));
    }

    #[tokio::test]
    async fn discovers_over_udp_from_a_unicast_responder() {
        let responder = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = responder.local_addr().unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 512];
            let (n, from) = responder.recv_from(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &build_query("octoprint", 0)[..]);
            let pkt = sample_response("Mon OctoPrint", 80, [127, 0, 0, 1], "/");
            responder.send_to(&pkt, from).await.unwrap();
        });
        let replies = discover_at(addr, "octoprint", Duration::from_millis(400), 1)
            .await
            .unwrap();
        assert_eq!(replies.len(), 1, "{replies:?}");
        assert_eq!(replies[0].full_address, "127.0.0.1");
        assert_eq!(replies[0].service_name, "Mon OctoPrint");
    }
}
