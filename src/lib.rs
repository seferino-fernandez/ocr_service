#[cfg(all(feature = "bundled-tesseract", feature = "system-tesseract"))]
compile_error!(
    "`bundled-tesseract` and `system-tesseract` are mutually exclusive; select one Tesseract build mode"
);

use std::io;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use opentelemetry_instrumentation_tower::HTTPLayer;
use tower::Layer as _;
use tower_http::normalize_path::NormalizePath;
use utoipa::OpenApi;
pub mod config;
pub mod middleware;
pub mod models;
pub mod ocr;
pub mod routes;
pub mod telemetry;
pub mod tessdata;

use config::app_config::AppConfig;
use middleware::{security, server};
use ocr::OcrEngine;
use tessdata::TessdataCatalog;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable as _};

/// Everything the handlers need, shared across requests.
///
/// Held behind `Arc`s because Axum's `State` extractor clones the state for every
/// request; cloning the catalog itself would copy the whole model set each time.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AppState {
    pub app_config: Arc<AppConfig>,
    pub catalog: Arc<TessdataCatalog>,
    pub ocr: Arc<OcrEngine>,
}

impl AppState {
    /// Builds the application state, discovering the Tesseract models on disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the configured tessdata directory cannot be read.
    pub fn from_config(app_config: AppConfig) -> io::Result<Self> {
        let catalog =
            TessdataCatalog::load(&app_config.tesseract, &app_config.service.default_language)?;
        let ocr = OcrEngine::new(&app_config.tesseract);

        Ok(Self {
            app_config: Arc::new(app_config),
            catalog: Arc::new(catalog),
            ocr: Arc::new(ocr),
        })
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "OCR Service",
        description = "API documentation for OCR Service",
    ),
    tags(
        (name = "System", description = "System Management APIs"),
        (name = "images", description = "Images API"),
        (name = "languages", description = "Languages API"),
    )
)]
struct ApiDoc;

/// The complete HTTP service, including URI normalization before route matching.
pub type AppRouter = NormalizePath<axum::Router>;

/// Assembles the HTTP router over an already-built [`AppState`].
///
/// Pure assembly: all fallible work happens in [`AppState::from_config`], so this cannot
/// fail and performs no I/O.
pub fn router(app_state: AppState) -> AppRouter {
    // Create the router with the routes and the OpenAPI documentation.
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api", routes::ImagesApi::router())
        .nest("/api", routes::LanguagesApi::router())
        .nest("/system", routes::ConfigApi::router())
        .nest("/system", routes::HealthApi::router())
        .split_for_parts();

    let body_limit_layer = if app_state.app_config.server.file_upload_max_size_enabled {
        Some(server::body_limit_layer(&app_state.app_config.server))
    } else {
        None
    };

    // Combine all the routes and apply the middleware layers.
    // The order of the layers is important. The first layer is the outermost layer.
    let mut router = router
        .merge(Scalar::with_url("/api-docs", api))
        .layer(security::cors_layer(&app_state.app_config.security))
        .layer(DefaultBodyLimit::disable());

    if let Some(limit_layer) = body_limit_layer {
        router = router.layer(limit_layer);
    }
    router = router.layer(server::timeout_layer(&app_state.app_config.server));

    let otel_metrics_layer = if !app_state.app_config.otel.sdk_disabled {
        Some(HTTPLayer::new())
    } else {
        None
    };

    if let Some(otel_metrics_layer) = otel_metrics_layer {
        router = router.layer(otel_metrics_layer);
    }
    // This must wrap the completed router: `Router::layer` runs after route matching,
    // which is too late for a middleware layer that rewrites request paths.
    server::normalize_path_layer().layer(router.with_state(app_state))
}
