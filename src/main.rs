use axum::BoxError;
use ocr_service::config::app_config::app_config;
use ocr_service::utils::telemetry::initialize_opentelemetry_providers;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    dotenvy::dotenv().ok();

    tracing::debug!("Initializing server configuration");
    let app_config = app_config().to_owned();

    // Initialize the OpenTelemetry Providers and hold the guard to keep them alive.
    let _guard = initialize_opentelemetry_providers(&app_config).await?;

    // Start the server.
    let server_address = format!("{}:{}", app_config.server.host, app_config.server.port);
    tracing::info!("Starting server on {}", server_address);
    let listener = TcpListener::bind(&server_address)
        .await
        .expect("Failed to bind address");
    let router = ocr_service::router(app_config);
    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_server())
        .await
        .expect("Failed to start server");
    Ok(())
}

async fn shutdown_server() {
    #[cfg(unix)]
    let sig_term = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Unable to register SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sig_term = std::future::pending::<()>();

    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Unable to register CTRL+C handler");
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = sig_term => {},
    }
}
