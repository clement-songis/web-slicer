//! Service d'authentification (T028) sur stockage SQLite réel : bootstrap du
//! premier admin, politiques d'inscription, invitations, identifiants.

use backend::adapters::storage::sqlite::SqliteStorage;
use backend::auth::{authenticate, register, AuthError};
use backend::domain::{RegistrationPolicy, Role, Storage};
use time::{Duration, OffsetDateTime};

async fn storage() -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("auth.db").display());
    let s = SqliteStorage::connect(&url).await.unwrap();
    (dir, s)
}

#[tokio::test]
async fn first_account_becomes_admin_and_bypasses_policy() {
    let (_d, s) = storage().await;
    s.instance()
        .set_registration_policy(RegistrationPolicy::Closed)
        .await
        .unwrap();

    // Malgré la politique fermée, le tout premier compte est créé (admin).
    let admin = register(&s, "boss@test.local", "password123", None)
        .await
        .expect("premier compte");
    assert_eq!(admin.role, Role::Admin);
    // Le mot de passe n'est jamais stocké en clair.
    assert!(!admin.password_hash.contains("password123"));
}

#[tokio::test]
async fn closed_policy_rejects_further_signups() {
    let (_d, s) = storage().await;
    register(&s, "admin@test.local", "password123", None)
        .await
        .unwrap();
    s.instance()
        .set_registration_policy(RegistrationPolicy::Closed)
        .await
        .unwrap();

    let err = register(&s, "intrus@test.local", "password123", None)
        .await
        .unwrap_err();
    assert!(matches!(err, AuthError::RegistrationClosed), "{err:?}");
}

#[tokio::test]
async fn invite_policy_requires_a_valid_token() {
    let (_d, s) = storage().await;
    let admin = register(&s, "admin@test.local", "password123", None)
        .await
        .unwrap();
    s.instance()
        .set_registration_policy(RegistrationPolicy::Invite)
        .await
        .unwrap();

    // Sans jeton → refus.
    assert!(matches!(
        register(&s, "a@test.local", "password123", None)
            .await
            .unwrap_err(),
        AuthError::InvalidInvitation
    ));

    // Avec un jeton valide → accepté, puis le jeton est consommé (usage unique).
    s.instance()
        .create_invitation(backend::domain::NewInvitation {
            token: "golden-ticket".into(),
            issued_by: admin.id,
            expires_at: OffsetDateTime::now_utc() + Duration::hours(24),
        })
        .await
        .unwrap();
    let user = register(&s, "guest@test.local", "password123", Some("golden-ticket"))
        .await
        .expect("inscription sur invitation");
    assert_eq!(user.role, Role::User);

    // Réutilisation du même jeton → refus.
    assert!(matches!(
        register(&s, "other@test.local", "password123", Some("golden-ticket"))
            .await
            .unwrap_err(),
        AuthError::InvalidInvitation
    ));
}

#[tokio::test]
async fn weak_password_and_duplicate_email_are_rejected() {
    let (_d, s) = storage().await;
    assert!(matches!(
        register(&s, "x@test.local", "short", None)
            .await
            .unwrap_err(),
        AuthError::WeakPassword
    ));
    register(&s, "dup@test.local", "password123", None)
        .await
        .unwrap();
    assert!(matches!(
        register(&s, "dup@test.local", "password123", None)
            .await
            .unwrap_err(),
        AuthError::EmailTaken
    ));
}

#[tokio::test]
async fn authenticate_checks_the_password() {
    let (_d, s) = storage().await;
    register(&s, "user@test.local", "password123", None)
        .await
        .unwrap();

    assert!(authenticate(&s, "user@test.local", "password123")
        .await
        .is_ok());
    assert!(matches!(
        authenticate(&s, "user@test.local", "wrong")
            .await
            .unwrap_err(),
        AuthError::InvalidCredentials
    ));
    assert!(matches!(
        authenticate(&s, "ghost@test.local", "password123")
            .await
            .unwrap_err(),
        AuthError::InvalidCredentials
    ));
}
