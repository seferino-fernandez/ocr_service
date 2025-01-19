use opentelemetry::global;
use utoipa::OpenApi;
pub mod config;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod utils;

use config::app_config::AppConfig;
use middleware::{security, server};
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable as _};

#[derive(Clone)]
#[non_exhaustive]
pub struct AppState {
    pub app_config: AppConfig,
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "OCR Service",
        description = "API documentation for OCR Service",
    ),
    tags(
        (name = "health", description = "Health API"),
        (name = "images", description = "Images API")
    )
)]
struct ApiDoc;

pub fn router(app_config: AppConfig) -> axum::Router {
    let app_state = AppState { app_config };

    // Create the router with the routes and the OpenAPI documentation.
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api", routes::ImagesApi::router())
        .nest("/system", routes::HealthApi::router())
        .split_for_parts();

    // Use `leak()` because the meter provider wants a static string (&str) but the service name is from an env variable.
    let global_meter =
        global::meter_provider().meter(app_state.app_config.service.name.clone().leak());

    let otel_metrics_layer = tower_otel_http_metrics::HTTPMetricsLayerBuilder::new()
        .with_meter(global_meter)
        .build()
        .unwrap();

    // Combine all the routes and apply the middleware layers.
    // The order of the layers is important. The first layer is the outermost layer.
    router
        .merge(Scalar::with_url("/api-docs", api))
        .layer(security::cors_layer(&app_state.app_config.security))
        .layer(server::body_limit_layer(&app_state.app_config.server))
        .layer(server::normalize_path_layer())
        .layer(server::timeout_layer(&app_state.app_config.server))
        .layer(otel_metrics_layer)
        .with_state(app_state)
}
