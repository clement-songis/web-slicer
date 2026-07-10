//! Service d'authentification : inscription (selon la politique d'instance),
//! bootstrap du premier admin, vérification des identifiants. Logique métier
//! pure au-dessus du trait `Storage` — aucune dépendance HTTP.

use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::domain::{
    Invitation, NewInvitation, NewUser, RegistrationPolicy, Role, Storage, StorageError, User,
    UserId, UserStatus,
};

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
    #[error("compte introuvable")]
    NotFound,
    #[error("impossible de supprimer le dernier administrateur")]
    LastAdmin,
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

/// Crée un compte géré par l'administrateur (rôle choisi, sans passer par la
/// politique d'inscription ni ouvrir de session). Pas de SMTP en v1 : l'admin
/// communique le mot de passe initial hors bande.
pub async fn create_managed_user(
    storage: &dyn Storage,
    email: &str,
    password: &str,
    role: Role,
) -> Result<User, AuthError> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(AuthError::WeakPassword);
    }
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

/// Réinitialise le mot de passe d'un compte (admin, pas de SMTP en v1).
/// `NotFound` remonte en erreur de stockage → l'appelant traduit.
pub async fn reset_password(
    storage: &dyn Storage,
    id: UserId,
    new_password: &str,
) -> Result<(), AuthError> {
    if new_password.len() < MIN_PASSWORD_LEN {
        return Err(AuthError::WeakPassword);
    }
    // Vérifie l'existence pour renvoyer NotFound plutôt qu'un UPDATE sans effet.
    storage.users().get(id).await.map_err(|e| match e {
        StorageError::NotFound => AuthError::NotFound,
        other => AuthError::Storage(other.to_string()),
    })?;
    let password_hash = hash_password(new_password)?;
    storage
        .users()
        .set_password_hash(id, &password_hash)
        .await?;
    Ok(())
}

/// Émet une invitation à usage unique valable `valid_days` jours (défaut 7).
pub async fn create_invitation(
    storage: &dyn Storage,
    issued_by: UserId,
    valid_days: i64,
) -> Result<Invitation, AuthError> {
    let token = Uuid::new_v4().simple().to_string();
    let expires_at = OffsetDateTime::now_utc() + Duration::days(valid_days.max(1));
    let invitation = storage
        .instance()
        .create_invitation(NewInvitation {
            token,
            issued_by,
            expires_at,
        })
        .await?;
    Ok(invitation)
}

/// Supprime un compte (cascade BDD via le repo). La purge fichiers est
/// orchestrée par l'appelant HTTP (side effect infra). Refuse de retirer le
/// dernier administrateur — sinon l'instance devient ingérable (edge case spec).
pub async fn delete_account(storage: &dyn Storage, id: UserId) -> Result<(), AuthError> {
    let target = storage.users().get(id).await.map_err(|e| match e {
        StorageError::NotFound => AuthError::NotFound,
        other => AuthError::Storage(other.to_string()),
    })?;

    if target.role == Role::Admin {
        let admins = storage
            .users()
            .list()
            .await?
            .into_iter()
            .filter(|u| u.role == Role::Admin)
            .count();
        if admins <= 1 {
            return Err(AuthError::LastAdmin);
        }
    }

    storage.users().delete(id).await?;
    Ok(())
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
