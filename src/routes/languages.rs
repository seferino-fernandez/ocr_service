use axum::Json;
use axum::extract::State;

use crate::AppState;
use crate::models::error::ErrorType;
use crate::models::languages::LanguagesResponse;

/// Fetch all of the available OCR Processing languages and models.
#[utoipa::path(
    get,
    operation_id = "get-available-languages",
    path = "/v1/languages",
    summary = "Fetch all of the available OCR Processing languages and models.",
    responses(
        (status = 200, description = "The available languages", body = LanguagesResponse, content_type = "application/json",
            example = json!({"languages": [{"language": "deu", "model": null}, {"language": "eng", "model": null}]})
        ),
   ),
    tag = "languages",
)]
#[tracing::instrument(skip(state))]
pub async fn languages(
    State(state): State<AppState>,
) -> Result<Json<LanguagesResponse>, ErrorType> {
    Ok(Json(LanguagesResponse {
        languages: state.catalog.list(),
    }))
}
