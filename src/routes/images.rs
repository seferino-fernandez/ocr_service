use crate::{
    AppState,
    models::{
        error::ErrorType,
        images::{ImagesForm, ImagesQueryParams, ImagesResponse},
    },
    ocr::OcrEngine,
};
use axum::{
    extract::{Multipart, Query, State},
    response::Json,
};

/// Perform OCR on an image
///
/// multipart: The multipart form data containing the image file.
/// language: (Optional) The language to use for the OCR. Defaults to "eng".
///
/// # Errors
///
/// - `InvalidRequest`: If the the file is not an image or the content type is not supported.
/// - `InternalError`: If something goes wrong while creating or using the OCR Engine.
#[utoipa::path(
    post,
    operation_id = "perform-ocr-on-image",
    path = "/v1/images",
    request_body(content = inline(ImagesForm), content_type = "multipart/form-data"),
    params(ImagesQueryParams),
    responses(
        (status = 200, description = "Text extracted from image successfully", body = ImagesResponse, content_type = "application/json",
            example = json!({"text": "The text that was extracted from your image!"})
        ),
   ),
    tag = "images",
)]
#[tracing::instrument(skip(state))]
pub async fn images(
    State(state): State<AppState>,
    Query(params): Query<ImagesQueryParams>,
    mut multipart: Multipart,
) -> Result<Json<ImagesResponse>, ErrorType> {
    tracing::debug!("Request received to perform OCR on image: {:?}", params);

    // Resolve the request's language and model to exactly one Tesseract model.
    let tesseract_model = state.catalog.resolve(&params)?;

    let field = multipart
        .next_field()
        .await?
        .ok_or_else(|| ErrorType::InvalidRequest("No image file provided".to_owned()))?;

    // Reject unsupported uploads before buffering the whole body.
    let content_type = field.content_type().ok_or_else(|| {
        ErrorType::InvalidRequest("No content type provided for given file".to_owned())
    })?;
    OcrEngine::check_content_type(content_type)?;

    let image_bytes = field.bytes().await?;

    let text = state.ocr.recognize(image_bytes, tesseract_model).await?;

    Ok(Json(ImagesResponse { text }))
}
