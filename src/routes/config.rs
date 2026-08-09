use axum::{Json, extract::State};
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::AppState;
use crate::models::config::ConfigResponse;
use crate::models::error::ErrorType;
use crate::ocr::{OcrEngine, OcrError};

#[derive(OpenApi)]
#[openapi(components(schemas(ConfigResponse)))]
pub struct ConfigApi;

impl ConfigApi {
    pub fn router() -> OpenApiRouter<AppState> {
        OpenApiRouter::with_openapi(ConfigApi::openapi()).routes(routes!(config))
    }
}

/// Fetch the upload and recognition settings available to API clients.
#[utoipa::path(
    get,
    operation_id = "get-application-config",
    path = "/config",
    summary = "Fetch the upload and recognition settings",
    responses(
        (status = 200, description = "The client-visible application settings", body = ConfigResponse, content_type = "application/json"),
    ),
    tag = "System",
)]
pub async fn config(State(app_state): State<AppState>) -> Result<Json<ConfigResponse>, ErrorType> {
    tracing::info!("GET /system/config");

    let server = &app_state.app_config.server;
    let service = &app_state.app_config.service;

    Ok(Json(ConfigResponse::new(
        server.file_upload_max_size,
        server.file_upload_max_size_enabled,
        server.timeout.as_secs(),
        service.default_language.clone(),
        allowed_file_types(),
    )))
}

/// Reads the OCR Engine's accepted content types through its validation interface.
///
/// The Engine owns this list; keeping the conversion here avoids a second, drifting copy
/// in the HTTP layer while `ALLOWED_FILE_TYPES` remains internal to the Engine.
fn allowed_file_types() -> Vec<String> {
    match OcrEngine::check_content_type("") {
        Err(OcrError::UnsupportedContentType { allowed, .. }) => {
            allowed.split(',').map(str::to_owned).collect()
        }
        Err(_) | Ok(()) => Vec::new(),
    }
}
