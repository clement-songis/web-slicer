//! Service d'authentification : inscription (selon la politique d'instance),
//! bootstrap du premier admin, vérification des identifiants. Logique métier
//! pure au-dessus du trait `Storage` — aucune dépendance HTTP.

use crate::domain::{NewUser, RegistrationPolicy, Role, Storage, StorageError, User, UserStatus};

use super::password::{hash_password, verify_password, PasswordError};

/// Erreur d'authentification.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("mot de passe trop court (8 caractères minimum)")]
    WeakPassword,
    #[error("cet email est déjà utilisé")]
    EmailTaken,
    #[error("les inscriptions sont fermées")]
    RegistrationClosed,
    #[error("invitation invalide ou expirée")]
    InvalidInvitation,
    #[error("identifiants invalides")]
    InvalidCredentials,
    #[error("compte désactivé")]
    AccountDisabled,
    #[error(transparent)]
    Password(#[from] PasswordError),
    #[error("erreur de stockage : {0}")]
    Storage(String),
}

impl From<StorageError> for AuthError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::Conflict(_) => AuthError::EmailTaken,
            other => AuthError::Storage(other.to_string()),
        }
    }
}

const MIN_PASSWORD_LEN: usize = 8;

/// Inscrit un compte. Le **premier** compte de l'instance devient `admin` et
/// contourne la politique (bootstrap) ; les suivants respectent
/// `registration_policy` (open/closed/invite).
pub async fn register(
    storage: &dyn Storage,
    email: &str,
    password: &str,
    invite_token: Option<&str>,
) -> Result<User, AuthError> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(AuthError::WeakPassword);
    }

    let is_first = storage.users().count().await? == 0;
    let role = if is_first {
        Role::Admin
    } else {
        enforce_policy(storage, invite_token).await?;
        Role::User
    };

    let password_hash = hash_password(password)?;
    let user = storage
        .users()
        .create(NewUser {
            email: email.to_string(),
            password_hash,
            role,
        })
        .await?;
    Ok(user)
}

/// Applique la politique d'inscription pour un compte non-bootstrap.
async fn enforce_policy(
    storage: &dyn Storage,
    invite_token: Option<&str>,
) -> Result<(), AuthError> {
    match storage.instance().settings().await?.registration_policy {
        RegistrationPolicy::Open => Ok(()),
        RegistrationPolicy::Closed => Err(AuthError::RegistrationClosed),
        RegistrationPolicy::Invite => {
            let token = invite_token.ok_or(AuthError::InvalidInvitation)?;
            storage
                .instance()
                .consume_invitation(token)
                .await
                .map_err(|_| AuthError::InvalidInvitation)?;
            Ok(())
        }
    }
}

/// Vérifie des identifiants et renvoie le compte si actif.
pub async fn authenticate(
    storage: &dyn Storage,
    email: &str,
    password: &str,
) -> Result<User, AuthError> {
    let user = storage
        .users()
        .find_by_email(email)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;

    if !verify_password(password, &user.password_hash) {
        return Err(AuthError::InvalidCredentials);
    }
    if user.status == UserStatus::Disabled {
        return Err(AuthError::AccountDisabled);
    }
    Ok(user)
}
