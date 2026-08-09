use crate::{
    AppState,
    models::{
        error::ErrorType,
        images::{ImagesForm, ImagesQueryParams, ImagesResponse},
    },
    ocr::{OcrEngine, RecognitionOptions, RecognitionOutput},
};
use axum::{
    extract::{Multipart, Query, State},
    http::header::CONTENT_TYPE,
    response::{IntoResponse, Json, Response},
};

/// Perform OCR on an image
///
/// multipart: The multipart form data containing the image file.
/// language: (Optional) The language to use for the OCR. Defaults to "eng".
/// model: (Optional) The Model to use for the Language.
/// psm: (Optional) Page segmentation mode, from 0 through 13.
/// oem: (Optional) OCR Engine mode, from 0 through 3.
/// dpi: (Optional) Positive source resolution in pixels per inch.
/// whitelist: (Optional) Characters to restrict Recognition to.
/// blacklist: (Optional) Characters to exclude from Recognition.
/// format: (Optional) Output format: text, hocr, alto, tsv, or box. Defaults to text.
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
        (status = 200, description = "Recognition completed successfully", content(
            (ImagesResponse = "application/json", example = json!({"text": "The text that was extracted from your image!", "confidence": 91})),
            (String = "text/vnd.hocr+html"),
            (String = "application/xml"),
            (String = "text/tab-separated-values"),
            (String = "text/plain")
        )),
   ),
    tag = "images",
)]
#[tracing::instrument(skip(state))]
pub async fn images(
    State(state): State<AppState>,
    Query(params): Query<ImagesQueryParams>,
    mut multipart: Multipart,
) -> Result<Response, ErrorType> {
    tracing::debug!("Request received to perform OCR on image: {:?}", params);

    // Resolve the request's language and model to exactly one Tesseract model.
    let tesseract_model = state.catalog.resolve(&params)?;
    let options = RecognitionOptions::try_new(
        params.psm,
        params.oem,
        params.dpi,
        params.whitelist,
        params.blacklist,
        params.format,
    )?;

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

    let output = state
        .ocr
        .recognize(image_bytes, tesseract_model, options)
        .await?;

    Ok(match output {
        RecognitionOutput::Text { text, confidence } => {
            Json(ImagesResponse { text, confidence }).into_response()
        }
        RecognitionOutput::Hocr(document) => {
            ([(CONTENT_TYPE, "text/vnd.hocr+html")], document).into_response()
        }
        RecognitionOutput::Alto(document) => {
            ([(CONTENT_TYPE, "application/xml")], document).into_response()
        }
        RecognitionOutput::Tsv(document) => {
            ([(CONTENT_TYPE, "text/tab-separated-values")], document).into_response()
        }
        RecognitionOutput::Box(document) => {
            ([(CONTENT_TYPE, "text/plain")], document).into_response()
        }
    })
}
