//! Validation de l'adresse d'un hôte d'impression (`print_host` d'Orca,
//! colonne `moonraker_url`) — audit 41, M1.
//!
//! Orca n'a pas de validation explicite, mais toute adresse passe par les
//! mêmes règles :
//! - `PhysicalPrinterDialog` retire les blancs de début/fin (`boost::trim`) ;
//! - `make_url` (OctoPrint.cpp:506, Moonraker.cpp:39) garde une adresse qui
//!   commence par `http://` / `https://` et préfixe `http://` sinon ;
//! - l'hôte et le port sont lus par libcurl (`Http::get_host_from_url`,
//!   `curl_url_set(CURLUPART_URL)`), qui refuse un hôte vide, des espaces
//!   (pas de `CURLU_ALLOW_SPACE`), un port non numérique ou > 65535.
//!
//! Une adresse que curl ne sait pas analyser ne peut pas fonctionner : Orca
//! échouerait au « Test ». Ici elle est refusée **à l'enregistrement** (422
//! lisible) au lieu d'être stockée puis de faire échouer l'envoi. L'analyseur
//! WHATWG (crate `url`, celui de reqwest) applique les mêmes refus que curl
//! sur ces points. Écarts volontaires, plus stricts qu'Orca :
//! - schéma autre que http/https refusé (Orca fabriquerait `http://ftp://…`) ;
//! - `?` et `#` refusés (make_url concatène le chemin après l'adresse :
//!   `http://h?x=1/api/version` serait faux) ;
//! - port 0 refusé ;
//! - schéma en majuscules accepté et ramené en minuscules (`HTTP://h` →
//!   `http://h`, que make_url d'Orca ne reconnaîtrait pas).
//!
//! L'adresse retenue est la saisie nettoyée (sans schéma ajouté) : même
//! valeur que le `print_host` d'Orca ; les clients appliquent make_url.

use reqwest::Url;

/// Motif de refus (code stable `details.reason` pour l'interface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostUrlError {
    /// Blancs à l'intérieur de l'adresse (« pas une url »).
    Whitespace,
    /// Schéma autre que http / https.
    Scheme,
    /// Hôte absent ou invalide (caractères interdits, IPv4 hors bornes…).
    Host,
    /// Port non numérique, nul ou > 65535.
    Port,
    /// Requête `?…` ou fragment `#…` (incompatibles avec make_url).
    QueryOrFragment,
}

impl HostUrlError {
    /// Code machine (`details.reason`).
    pub fn reason(self) -> &'static str {
        match self {
            Self::Whitespace => "whitespace",
            Self::Scheme => "scheme",
            Self::Host => "host",
            Self::Port => "port",
            Self::QueryOrFragment => "query",
        }
    }

    /// Message lisible (FR-032), avec des exemples d'adresses valides.
    pub fn message(self, raw: &str) -> String {
        let why = match self {
            Self::Whitespace => "ce n'est pas une adresse (espaces interdits)",
            Self::Scheme => "seuls http:// et https:// sont acceptés",
            Self::Host => "nom d'hôte ou adresse IP invalide",
            Self::Port => "port invalide (nombre de 1 à 65535)",
            Self::QueryOrFragment => "l'adresse ne doit contenir ni « ? » ni « # »",
        };
        format!(
            "Adresse de l'hôte invalide « {} » : {why}. \
             Exemples : 192.168.1.20:7125, voron.local ou http://voron.local:7125",
            raw.trim()
        )
    }
}

/// URL complète (`make_url` d'Orca sans chemin) : `http://` ajouté sans schéma.
pub fn with_scheme(host: &str) -> String {
    let head = host.get(..8).unwrap_or(host).to_ascii_lowercase();
    if head.starts_with("http://") || head.starts_with("https://") {
        host.to_string()
    } else {
        format!("http://{host}")
    }
}

/// Valide et nettoie une adresse d'hôte. `Ok(None)` = champ vide
/// (imprimante non connectée, comme un `print_host` vide chez Orca).
pub fn normalize_host_url(raw: &str) -> Result<Option<String>, HostUrlError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(HostUrlError::Whitespace);
    }
    // Schéma explicite : http / https seulement, ramené en minuscules. Sans
    // `://`, la saisie est un hôte (`h:7125`), comme pour curl après le
    // préfixe `http://` d'Orca.
    let cleaned = match trimmed.find("://") {
        Some(at) => {
            let scheme = trimmed[..at].to_ascii_lowercase();
            if scheme != "http" && scheme != "https" {
                return Err(HostUrlError::Scheme);
            }
            format!("{scheme}{}", &trimmed[at..])
        }
        None => trimmed.to_string(),
    };
    if cleaned.contains('?') || cleaned.contains('#') {
        return Err(HostUrlError::QueryOrFragment);
    }
    let url = Url::parse(&with_scheme(&cleaned)).map_err(|e| match e {
        url::ParseError::InvalidPort => HostUrlError::Port,
        _ => HostUrlError::Host,
    })?;
    if url.host_str().is_none_or(str::is_empty) {
        return Err(HostUrlError::Host);
    }
    if url.port() == Some(0) {
        return Err(HostUrlError::Port);
    }
    Ok(Some(cleaned))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_addresses_orca_accepts() {
        for (raw, kept) in [
            ("192.168.1.20", "192.168.1.20"),
            ("192.168.1.20:7125", "192.168.1.20:7125"),
            ("  voron.local:7125 ", "voron.local:7125"),
            ("http://voron.local", "http://voron.local"),
            ("https://connect.prusa3d.com", "https://connect.prusa3d.com"),
            ("http://h:7125/moonraker/", "http://h:7125/moonraker/"),
            ("HTTP://Voron.local", "http://Voron.local"),
            ("http://user:pass@octo.local", "http://user:pass@octo.local"),
            ("[fe80::1]:7125", "[fe80::1]:7125"),
            ("localhost", "localhost"),
            ("my_printer.lan", "my_printer.lan"),
        ] {
            assert_eq!(normalize_host_url(raw), Ok(Some(kept.to_string())), "{raw}");
        }
        assert_eq!(normalize_host_url("   "), Ok(None));
        assert_eq!(normalize_host_url(""), Ok(None));
    }

    #[test]
    fn refuses_what_curl_cannot_parse() {
        for (raw, why) in [
            ("pas une url", HostUrlError::Whitespace),
            ("http://pas une url", HostUrlError::Whitespace),
            ("ftp://h", HostUrlError::Scheme),
            ("ws://h:7125", HostUrlError::Scheme),
            ("http://", HostUrlError::Host),
            ("http://:7125", HostUrlError::Host),
            ("192.168.1.300", HostUrlError::Host),
            ("h<o>st", HostUrlError::Host),
            ("h:99999", HostUrlError::Port),
            ("h:abc", HostUrlError::Port),
            ("h:0", HostUrlError::Port),
            ("http://h/?x=1", HostUrlError::QueryOrFragment),
            ("h#frag", HostUrlError::QueryOrFragment),
        ] {
            assert_eq!(normalize_host_url(raw), Err(why), "{raw}");
        }
    }

    #[test]
    fn with_scheme_is_orca_make_url_without_path() {
        assert_eq!(with_scheme("h:7125"), "http://h:7125");
        assert_eq!(with_scheme("https://h"), "https://h");
        assert_eq!(with_scheme("HTTPS://h"), "HTTPS://h");
        assert_eq!(with_scheme("http://h/"), "http://h/");
    }

    #[test]
    fn messages_are_readable() {
        let m = HostUrlError::Whitespace.message(" pas une url ");
        assert!(
            m.contains("« pas une url »") && m.contains("192.168.1.20:7125"),
            "{m}"
        );
        assert_eq!(HostUrlError::Port.reason(), "port");
    }
}
