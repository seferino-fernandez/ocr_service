use axum::Json;

use crate::models::error::ErrorType;
use crate::models::health::HealthResponse;

/// Fetch the health of the system.
#[utoipa::path(
    get,
    operation_id = "get-system-health",
    path = "/health",
    summary = "Fetch the health of the system",
    responses(
        (status = 200, description = "The status of the system", body = HealthResponse, content_type = "application/json",
            example = json!({"status": "ok"})
        ),
   ),
    tag = "health",
)]
pub async fn health() -> Result<Json<HealthResponse>, ErrorType> {
    Ok(Json(HealthResponse::new("ok")))
}
