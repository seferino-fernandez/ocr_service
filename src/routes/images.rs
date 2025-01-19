use std::{
    env::current_dir,
    io::{Cursor, Error},
    path::PathBuf,
};

use axum::{
    extract::{Multipart, Query},
    Json,
};

use image::ImageReader;
use tesseract_rs::TesseractAPI;

use crate::models::{
    error::ErrorType,
    images::{ImagesForm, ImagesQueryParams, ImagesResponse},
};

use crate::utils::validations::{validate_file_type, validate_language};

const DEFAULT_OCR_LANGUAGE: &str = "eng";
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
    Query(params): Query<ImagesQueryParams>,
    mut multipart: Multipart,
) -> Result<Json<ImagesResponse>, ErrorType> {
    tracing::debug!("Request received to perform OCR on image");
    let mut ocr_language = DEFAULT_OCR_LANGUAGE.to_owned();
    // Validate the language if it was provided
    if let Some(language) = params.language {
        validate_language(&language)?;
        ocr_language = language;
    } else {
        tracing::debug!("No language provided, defaulting to {}", ocr_language);
    }

    let field = multipart
        .next_field()
        .await
        .map_err(|multipart_error| ErrorType::InvalidRequest(multipart_error.to_string()))?
        .ok_or_else(|| ErrorType::InvalidRequest("No image file provided".to_owned()))?;

    if let Some(content_type) = field.content_type() {
        validate_file_type(content_type)?;
        tracing::debug!("Valid content type: {}", content_type);
    } else {
        return Err(ErrorType::InvalidRequest(
            "No content type provided for given file".to_owned(),
        ));
    }

    let image_data = field
        .bytes()
        .await
        .map_err(|multipart_error| ErrorType::InvalidRequest(multipart_error.to_string()))?;
    tracing::debug!("Retrieved image data");

    let img = ImageReader::new(Cursor::new(image_data))
        .with_guessed_format()
        .map_err(|error| ErrorType::InvalidRequest(error.to_string()))?
        .decode()
        .map_err(|image_error| ErrorType::InvalidRequest(image_error.to_string()))?;
    tracing::debug!("Decoded image successfully");

    // Convert the image to RGB8 and gather image dimensions for Tesseract
    let rgb_image = img.to_rgb8();
    let (width, height) = rgb_image.dimensions();
    let bytes_per_line = (width * BYTES_PER_PIXEL).try_into().map_err(|error| {
        ErrorType::InvalidRequest(format!("Image dimensions are too large: {error}"))
    })?;
    let raw_image_data = rgb_image.into_raw();
    tracing::debug!("Successfully converted to raw image data.");

    let tesseract_api = TesseractAPI::new();
    tracing::debug!("Initialized Tesseract");
    let resource_path: PathBuf = current_dir()
        .map_err(|error: Error| {
            ErrorType::InternalError(anyhow::anyhow!("Failed to get current directory: {error}"))
        })?
        .join("tesseract");
    tracing::debug!(
        "Using language {} and resource path {} for Tesseract",
        ocr_language,
        resource_path.to_str().unwrap()
    );
    tesseract_api
        .init(resource_path.to_str().unwrap(), &ocr_language)
        .map_err(|tess_error| {
            ErrorType::InternalError(anyhow::anyhow!(
                "Something went wrong while performing OCR: {tess_error}"
            ))
        })?;
    tracing::debug!("Initialized Tesseract with resource path");
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
    tracing::debug!("Set image in Tesseract");
    let text = tesseract_api.get_utf8_text().map_err(|tess_error| {
        ErrorType::InvalidRequest(format!(
            "Something went wrong while extracting the text: {tess_error}"
        ))
    })?;
    Ok(Json(ImagesResponse { text }))
}
