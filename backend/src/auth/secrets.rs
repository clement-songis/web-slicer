//! Chiffrement au repos des secrets d'instance (T075, FR-060) : les clés API
//! Moonraker sont chiffrées avant d'être stockées et ne circulent jamais en
//! clair vers le client. Chiffrement authentifié ChaCha20-Poly1305 ; la clé
//! d'instance vient de `INSTANCE_SECRET_KEY` (base64 de 32 octets).

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, KeyInit, Nonce};

/// Nonce ChaCha20-Poly1305 : 12 octets, préfixés au chiffré.
const NONCE_LEN: usize = 12;

/// Clé de repli **de développement** (déterministe pour que les secrets déjà
/// stockés restent déchiffrables entre deux redémarrages sans configuration).
/// Un déploiement réel **doit** définir `INSTANCE_SECRET_KEY`.
const DEV_KEY: [u8; 32] = *b"web-slicer-dev-instance-key-0001";

/// Coffre symétrique : chiffre/déchiffre les secrets d'instance.
#[derive(Clone)]
pub struct SecretBox {
    key: [u8; 32],
}

impl SecretBox {
    /// Construit un coffre à partir d'une clé de 32 octets.
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Charge la clé depuis `INSTANCE_SECRET_KEY` (base64 de 32 octets) ; à
    /// défaut, retombe sur la clé de développement avec un avertissement.
    pub fn from_env() -> Self {
        match std::env::var("INSTANCE_SECRET_KEY") {
            Ok(encoded) => match STANDARD.decode(encoded.trim()) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&bytes);
                    Self::new(key)
                }
                _ => {
                    tracing::warn!(
                        "INSTANCE_SECRET_KEY invalide (base64 de 32 octets attendu) ; \
                         repli sur la clé de développement"
                    );
                    Self::new(DEV_KEY)
                }
            },
            Err(_) => {
                tracing::warn!(
                    "INSTANCE_SECRET_KEY absente ; repli sur la clé de développement \
                     (ne pas utiliser en production)"
                );
                Self::new(DEV_KEY)
            }
        }
    }

    fn cipher(&self) -> ChaCha20Poly1305 {
        ChaCha20Poly1305::new_from_slice(&self.key).expect("clé d'instance de 32 octets")
    }

    /// Chiffre un secret : renvoie `base64(nonce ‖ chiffré+tag)`.
    pub fn encrypt(&self, plaintext: &str) -> String {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher()
            .encrypt(&nonce, plaintext.as_bytes())
            .expect("chiffrement ChaCha20-Poly1305");
        let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        blob.extend_from_slice(nonce.as_ref());
        blob.extend_from_slice(&ciphertext);
        STANDARD.encode(blob)
    }

    /// Déchiffre un jeton produit par [`encrypt`](Self::encrypt). Renvoie `None`
    /// si le format, l'authentification ou l'UTF-8 échouent (jamais de panique).
    pub fn decrypt(&self, token: &str) -> Option<String> {
        let blob = STANDARD.decode(token.trim()).ok()?;
        if blob.len() < NONCE_LEN {
            return None;
        }
        let (nonce, ciphertext) = blob.split_at(NONCE_LEN);
        let mut nonce_bytes = [0u8; NONCE_LEN];
        nonce_bytes.copy_from_slice(nonce);
        let nonce = Nonce::from(nonce_bytes);
        let plaintext = self.cipher().decrypt(&nonce, ciphertext).ok()?;
        String::from_utf8(plaintext).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_secret() {
        let vault = SecretBox::new([7u8; 32]);
        let token = vault.encrypt("moonraker-api-key");
        // Le jeton ne contient jamais le clair.
        assert!(!token.contains("moonraker-api-key"));
        assert_eq!(vault.decrypt(&token).as_deref(), Some("moonraker-api-key"));
    }

    #[test]
    fn distinct_ciphertexts_for_same_plaintext() {
        // Nonce aléatoire → deux chiffrés différents.
        let vault = SecretBox::new([9u8; 32]);
        assert_ne!(vault.encrypt("same"), vault.encrypt("same"));
    }

    #[test]
    fn wrong_key_or_garbage_fails_cleanly() {
        let a = SecretBox::new([1u8; 32]);
        let b = SecretBox::new([2u8; 32]);
        let token = a.encrypt("secret");
        // Mauvaise clé → échec d'authentification, pas de panique.
        assert!(b.decrypt(&token).is_none());
        // Jeton corrompu → None.
        assert!(a.decrypt("pas-du-base64!!").is_none());
        assert!(a.decrypt("").is_none());
    }
}
