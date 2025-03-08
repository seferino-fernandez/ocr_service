use crate::{
    models::{
        error::ErrorType,
        images::{ImagesForm, ImagesQueryParams, ImagesResponse},
    },
    utils::validations::{validate_file_type, validate_language_params},
    AppState,
};
use axum::{
    extract::{Multipart, Query, State},
    response::Json,
};
use image::ImageReader;
use std::io::Cursor;
use std::path::PathBuf;
use tesseract_rs::TesseractAPI;

const BYTES_PER_PIXEL: u32 = 3;

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
#[tracing::instrument]
pub async fn images(
    State(state): State<AppState>,
    Query(params): Query<ImagesQueryParams>,
    mut multipart: Multipart,
) -> Result<Json<ImagesResponse>, ErrorType> {
    tracing::debug!("Request received to perform OCR on image: {:?}", params);
    let default_language = state.app_config.service.default_language.to_owned();

    // Validate language parameters and get appropriate TesseractModel
    let tesseract_model = validate_language_params(
        &params,
        &state.available_tesseract_languages,
        &default_language,
    )?;

    // Log which language we're using
    tracing::debug!(
        "Using language {} for OCR",
        if let Some(model) = &tesseract_model.model {
            format!("{} (model: {})", tesseract_model.language, model)
        } else {
            tesseract_model.language.clone()
        }
    );

    let field = multipart
        .next_field()
        .await
        .map_err(|multipart_error| ErrorType::InvalidRequest(multipart_error.to_string()))?
        .ok_or_else(|| ErrorType::InvalidRequest("No image file provided".to_owned()))?;

    if let Some(content_type) = field.content_type() {
        validate_file_type(content_type)?;
    } else {
        return Err(ErrorType::InvalidRequest(
            "No content type provided for given file".to_owned(),
        ));
    }

    let file_content = field
        .bytes()
        .await
        .map_err(|extract_error| ErrorType::InvalidRequest(extract_error.to_string()))?;

    // Instantiate the Tesseract API
    let tesseract_api = TesseractAPI::new();

    // Get the $TESSDATA_PATH environment variable stored in the AppConfig
    let resource_path = PathBuf::from(&state.app_config.tesseract.data_path);

    tracing::debug!(
        "Using language {} and resource path {}",
        tesseract_model.language,
        resource_path.to_str().unwrap_or_default()
    );

    let img = ImageReader::new(Cursor::new(file_content))
        .with_guessed_format()
        .map_err(|error| ErrorType::InvalidRequest(error.to_string()))?
        .decode()
        .map_err(|image_error| ErrorType::InvalidRequest(image_error.to_string()))?;

    // Convert the image to RGB8 and gather image dimensions for Tesseract
    let rgb_image = img.to_rgb8();
    let (width, height) = rgb_image.dimensions();
    let bytes_per_line = (width * BYTES_PER_PIXEL).try_into().map_err(|error| {
        ErrorType::InvalidRequest(format!("Image dimensions are too large: {error}"))
    })?;
    let raw_image_data = rgb_image.into_raw();
    let language_model_path = tesseract_model.relative_path.unwrap_or_default();

    tracing::debug!(
        "Initializing Tesseract API with path: {} and language: {}",
        resource_path.to_str().unwrap_or_default(),
        language_model_path
    );
    tesseract_api
        .init(
            resource_path.to_str().unwrap_or_default(),
            language_model_path.as_str(),
        )
        .map_err(|tess_error| {
            ErrorType::InternalError(anyhow::anyhow!(
                "Something went wrong while performing OCR: {tess_error}"
            ))
        })?;

    tesseract_api
        .set_image(
            &raw_image_data,
            width.try_into().map_err(|error| {
                ErrorType::InvalidRequest(format!("Image width is too large: {error}"))
            })?,
            height.try_into().map_err(|error| {
                ErrorType::InvalidRequest(format!("Image height is too large: {error}"))
            })?,
            BYTES_PER_PIXEL.try_into().unwrap(),
            bytes_per_line,
        )
        .map_err(|tess_error| {
            ErrorType::InternalError(anyhow::anyhow!(
                "Something went wrong while processing the image: {tess_error}"
            ))
        })?;

    let text = tesseract_api.get_utf8_text().map_err(|tess_error| {
        ErrorType::InvalidRequest(format!(
            "Something went wrong while extracting the text: {tess_error}"
        ))
    })?;

    Ok(Json(ImagesResponse { text }))
}
