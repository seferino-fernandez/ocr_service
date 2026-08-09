//! `ocr-service`

#![warn(missing_docs)]
#![forbid(unsafe_code)]

use axum::BoxError;
use ocr_service::AppState;
use ocr_service::config::app_config::app_config;
use ocr_service::telemetry::initialize_opentelemetry_providers;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // .env is used only for development, so a missing file is not an error.
    let _dotenv = dotenvy::dotenv();

    tracing::debug!("Initializing server configuration");
    let app_config = app_config().to_owned();

    // Initialize the OpenTelemetry Providers and hold the guard to keep them alive.
    let _guard = initialize_opentelemetry_providers(&app_config).await?;

    // Start the server.
    let server_address = format!("{}:{}", app_config.server.host, app_config.server.port);
    tracing::info!("Starting server on {}", server_address);
    let listener = TcpListener::bind(&server_address).await?;
    let router = ocr_service::router(AppState::from_config(app_config)?);
    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_server())
        .await?;
    Ok(())
}

async fn shutdown_server() {
    // If a handler cannot be registered, log it and leave that shutdown path disabled
    // rather than aborting a server that is otherwise healthy.
    #[cfg(unix)]
    let sig_term = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!("Unable to register SIGTERM handler: {error}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let sig_term = std::future::pending::<()>();

    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!("Unable to register CTRL+C handler: {error}");
            std::future::pending::<()>().await;
        }
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = sig_term => {},
    }
}
