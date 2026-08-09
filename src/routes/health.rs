use axum::Json;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::AppState;
use crate::models::error::ErrorType;
use crate::models::health::HealthResponse;

#[derive(OpenApi)]
#[openapi(components(schemas(HealthResponse)))]
pub struct HealthApi;

impl HealthApi {
    pub fn router() -> OpenApiRouter<AppState> {
        OpenApiRouter::with_openapi(HealthApi::openapi()).routes(routes!(health))
    }
}

/// Fetch the health of the application.
#[utoipa::path(
    get,
    operation_id = "get-application-health",
    path = "/health",
    summary = "Fetch the health of the application",
    responses(
        (status = 200, description = "The status of the application", body = HealthResponse, content_type = "application/json",
            example = json!({"status": "ok"})
        ),
   ),
    tag = "system",
)]
pub async fn health() -> Result<Json<HealthResponse>, ErrorType> {
    tracing::info!("GET /system/health");
    Ok(Json(HealthResponse::new("ok")))
}
