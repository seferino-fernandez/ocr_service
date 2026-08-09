//! `ocr-service`

#![warn(missing_docs)]
#![forbid(unsafe_code)]

use axum::{BoxError, ServiceExt, extract::Request};
use ocr_service::AppState;
use ocr_service::config::app_config::app_config;
use ocr_service::telemetry::initialize_opentelemetry_providers;
use tokio::net::TcpListener;

fn main() -> Result<(), BoxError> {
    // .env is used only for development, so a missing file is not an error.
    let _dotenv = dotenvy::dotenv();

    let app_config = app_config().to_owned();
    let max_blocking_threads = app_config.server.max_blocking_threads;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(max_blocking_threads)
        .build()?;

    runtime.block_on(run(app_config))
}

async fn run(app_config: ocr_service::config::app_config::AppConfig) -> Result<(), BoxError> {
    tracing::debug!("Initializing server configuration");

    // Initialize the OpenTelemetry Providers and hold the guard to keep them alive.
    let _guard = initialize_opentelemetry_providers(&app_config).await?;
    tracing::info!(
        max_blocking_threads = app_config.server.max_blocking_threads,
        "Configured Tokio blocking pool"
    );

    // Start the server.
    let server_address = format!("{}:{}", app_config.server.host, app_config.server.port);
    tracing::info!("Starting server on {}", server_address);
    let listener = TcpListener::bind(&server_address).await?;
    let router = ocr_service::router(AppState::from_config(app_config)?);
    axum::serve(listener, ServiceExt::<Request>::into_make_service(router))
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
