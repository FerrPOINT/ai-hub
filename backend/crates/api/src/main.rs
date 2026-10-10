use aihub_api::{ApiDoc, AppState, router};
use aihub_application::{Foundation, FoundationStore};
use aihub_infrastructure::{auth::CentralAuth, config::Config, postgres::PgStore, vault::Vault};
use std::sync::Arc;
use utoipa::OpenApi;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = std::env::args().nth(1).unwrap_or_else(|| "serve".into());
    if command == "export-openapi" {
        let document = ApiDoc::openapi().to_pretty_json()?;
        if let Some(path) = std::env::args().nth(2) {
            std::fs::write(path, document)?;
        } else {
            println!("{document}");
        }
        return Ok(());
    }
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::new("aihub=info"))
        .init();
    let config = Config::load()?;
    let vault = Vault::new(config.vault_key.to_vec())?;
    let store = Arc::new(
        PgStore::connect(
            &config.database_url,
            config.installation_id,
            vault.fingerprint(),
        )
        .await?,
    );
    match command.as_str() {
        "migrate" => {
            store.migrate().await?;
            return Ok(());
        }
        "initialize" => {
            let stable_key = std::env::var("AIHUB_STABLE_KEY")
                .map_err(|_| "AIHUB_STABLE_KEY required for explicit initialization")?;
            store.initialize(&stable_key).await?;
            return Ok(());
        }
        "serve" => (),
        _ => {
            return Err(
                "unknown command; expected serve, migrate, initialize, export-openapi".into(),
            );
        }
    }
    store.ready().await?;
    let state = AppState {
        foundation: Foundation {
            installation_id: config.installation_id,
            auth: Arc::new(CentralAuth::default()),
            store,
        },
        external_calls: config.external_calls,
        auth_issuer: config.auth_issuer.to_string().trim_end_matches('/').into(),
        public_origin: config.public_origin.to_string(),
        admin_origin: config.admin_origin.map(|origin| origin.to_string()),
    };
    let app = router(state, config.max_body_bytes);
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    tracing::info!(installation=%config.installation_id,"AI Hub listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
