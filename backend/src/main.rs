//! Point d'entrée du serveur Web-Slicer. Lit la configuration depuis
//! l'environnement, construit l'application (voir `server::build_app`) et sert
//! via `axum::serve`.

use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=info,tower_http=info".into()),
        )
        .init();

    let data_dir: PathBuf = std::env::var("DATA_DIR")
        .unwrap_or_else(|_| "data".into())
        .into();
    std::fs::create_dir_all(&data_dir)?;

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| format!("sqlite://{}", data_dir.join("web-slicer.db").display()));
    let bind = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());

    let app = backend::server::build_app(&database_url, data_dir).await?;

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("web-slicer backend en écoute sur http://{bind}");
    axum::serve(listener, app).await?;
    Ok(())
}
