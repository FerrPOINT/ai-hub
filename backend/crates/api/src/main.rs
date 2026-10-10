use aihub_api::{ApiDoc, AppState, router};
use aihub_application::{Foundation, FoundationStore};
use aihub_infrastructure::{
    auth::CentralAuth, config::Config, maintenance::MaintenanceWorker, postgres::PgStore,
    vault::Vault,
};
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
    let vault = Arc::new(Vault::new(config.vault_key.to_vec())?);
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
    let maintenance = MaintenanceWorker::new(store.clone(), vault.clone())?;
    let state = AppState {
        foundation: Foundation {
            installation_id: config.installation_id,
            auth: Arc::new(CentralAuth::default()),
            store,
            bindings: vault,
        },
        external_calls: config.external_calls,
        auth_issuer: config.auth_issuer.to_string().trim_end_matches('/').into(),
        public_origin: config.public_origin.to_string(),
        admin_origin: config.admin_origin.map(|origin| origin.to_string()),
    };
    let app = router(state, config.max_body_bytes);
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    let (stop, receiver) = tokio::sync::watch::channel(false);
    let mut worker = tokio::spawn(maintenance.run(
        std::time::Duration::from_secs(config.maintenance_tick_seconds),
        receiver,
    ));
    tracing::info!(installation=%config.installation_id,"AI Hub listening");
    let shutdown_stop = stop.clone();
    let served = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            let _ = shutdown_stop.send(true);
        })
        .await;
    let _ = stop.send(true);
    match tokio::time::timeout(std::time::Duration::from_secs(5), &mut worker).await {
        Ok(Ok(Ok(()))) => (),
        Ok(_) => tracing::warn!("AI Hub maintenance stopped with an error"),
        Err(_) => {
            worker.abort();
            let _ = worker.await;
            tracing::warn!("AI Hub maintenance shutdown timed out; database transactions dropped");
        }
    }
    served?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM handler");
        tokio::select! {_=tokio::signal::ctrl_c()=>(),_=terminate.recv()=>()}
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
