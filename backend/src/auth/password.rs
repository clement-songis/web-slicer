//! Hachage de mots de passe (argon2id). Le mot de passe en clair ne quitte
//! jamais cette frontière : seul le hash PHC est stocké (data-model.md).

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

/// Erreur de hachage/vérification.
#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("échec du hachage : {0}")]
    Hash(String),
}

/// Hache un mot de passe avec argon2id (sel aléatoire, format PHC).
pub fn hash_password(plaintext: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plaintext.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| PasswordError::Hash(e.to_string()))
}

/// Vérifie un mot de passe contre un hash PHC. `false` si le hash est illisible
/// ou ne correspond pas (jamais d'erreur exposée à l'appelant).
pub fn verify_password(plaintext: &str, phc_hash: &str) -> bool {
    match PasswordHash::new(phc_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(plaintext.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_argon2id_and_never_contains_the_plaintext() {
        let hash = hash_password("s3cr3t-p@ss").unwrap();
        assert!(
            hash.starts_with("$argon2id$"),
            "algorithme argon2id : {hash}"
        );
        assert!(!hash.contains("s3cr3t-p@ss"), "le clair n'apparaît jamais");
    }

    #[test]
    fn verify_accepts_the_right_password_and_rejects_others() {
        let hash = hash_password("correct horse").unwrap();
        assert!(verify_password("correct horse", &hash));
        assert!(!verify_password("wrong horse", &hash));
        assert!(!verify_password("correct horse", "pas-un-hash-phc"));
    }

    #[test]
    fn same_password_yields_distinct_hashes() {
        // sel aléatoire → deux hachages différents pour le même mot de passe.
        assert_ne!(hash_password("dup").unwrap(), hash_password("dup").unwrap());
    }
}
