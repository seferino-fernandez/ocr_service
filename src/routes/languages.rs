use axum::extract::State;
use axum::Json;

use crate::models::error::ErrorType;
use crate::models::languages::{LanguagesResponse, TesseractModel};
use crate::AppState;

/// Fetch all of the available OCR Processing languages and models.
#[utoipa::path(
    get,
    operation_id = "get-available-languages",
    path = "/v1/languages",
    summary = "Fetch all of the available OCR Processing languages and models.",
    responses(
        (status = 200, description = "The available languages", body = LanguagesResponse, content_type = "application/json",
            example = json!({"languages": [{"language": "eng", "model": "eng.traineddata"}, {"language": "deu", "model": "deu.traineddata"}]})
        ),
   ),
    tag = "languages",
)]
#[tracing::instrument]
pub async fn languages(
    State(state): State<AppState>,
) -> Result<Json<LanguagesResponse>, ErrorType> {
    let available_languages = state.available_tesseract_languages;
    // Sort the languages by language name then by model name
    let mut sorted_languages: Vec<TesseractModel> = available_languages.into_iter().collect();
    sorted_languages.sort_by(|a, b| a.language.cmp(&b.language).then(a.model.cmp(&b.model)));
    Ok(Json(LanguagesResponse {
        languages: sorted_languages,
    }))
}
