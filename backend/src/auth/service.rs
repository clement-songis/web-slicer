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
    #[error("mot de passe actuel incorrect")]
    WrongPassword,
    #[error("le nouveau mot de passe doit différer de l'actuel")]
    SamePassword,
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

/// Change le mot de passe du compte connecté (lot 32) : l'ancien mot de passe
/// est exigé, le nouveau doit respecter la longueur minimale et différer de
/// l'actuel. Renvoie le compte relu (nouveau hash) : l'appelant en dérive
/// l'empreinte de session (`session_stamp`), ce qui invalide toutes les autres
/// sessions du compte.
pub async fn change_password(
    storage: &dyn Storage,
    user: &User,
    current_password: &str,
    new_password: &str,
) -> Result<User, AuthError> {
    if !verify_password(current_password, &user.password_hash) {
        return Err(AuthError::WrongPassword);
    }
    if new_password.len() < MIN_PASSWORD_LEN {
        return Err(AuthError::WeakPassword);
    }
    if new_password == current_password {
        return Err(AuthError::SamePassword);
    }
    let password_hash = hash_password(new_password)?;
    storage
        .users()
        .set_password_hash(user.id, &password_hash)
        .await?;
    storage.users().get(user.id).await.map_err(|e| match e {
        StorageError::NotFound => AuthError::NotFound,
        other => AuthError::Storage(other.to_string()),
    })
}

/// Empreinte de session dérivée du hash du mot de passe (sel argon2 compris,
/// donc différente à chaque changement). Posée dans la session à la connexion
/// et comparée à chaque requête : un changement ou une réinitialisation de mot
/// de passe invalide d'un coup toutes les sessions ouvertes avant, quel que
/// soit le store (SQLite, Postgres, mémoire). Ce n'est pas un secret : seul
/// un changement de valeur compte.
pub fn session_stamp(password_hash: &str) -> String {
    use md5::{Digest, Md5};
    Md5::digest(password_hash.as_bytes())
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
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

/// Désactive ou réactive un compte (lot 35). Refuse de désactiver le
/// dernier administrateur **actif** (même garde que la suppression) ; une
/// désactivation invalide immédiatement les sessions ouvertes (l'extracteur
/// `CurrentUser` vérifie le statut à chaque requête).
pub async fn set_account_status(
    storage: &dyn Storage,
    id: UserId,
    status: UserStatus,
) -> Result<(), AuthError> {
    let target = storage.users().get(id).await.map_err(|e| match e {
        StorageError::NotFound => AuthError::NotFound,
        other => AuthError::Storage(other.to_string()),
    })?;

    if status == UserStatus::Disabled
        && target.role == Role::Admin
        && target.status == UserStatus::Active
    {
        let active_admins = storage
            .users()
            .list()
            .await?
            .into_iter()
            .filter(|u| u.role == Role::Admin && u.status == UserStatus::Active)
            .count();
        if active_admins <= 1 {
            return Err(AuthError::LastAdmin);
        }
    }

    storage.users().set_status(id, status).await?;
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
