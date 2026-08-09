use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[non_exhaustive]
pub struct ImagesResponse {
    /// The text extracted from the image.
    pub text: String,
    /// Tesseract's mean word confidence, from 0 through 100.
    #[schema(minimum = 0, maximum = 100)]
    pub confidence: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
#[expect(
    dead_code,
    reason = "the field exists so utoipa can describe the multipart body in the OpenAPI schema; \
              the multipart payload itself is read through the Multipart extractor"
)]
#[non_exhaustive]
pub struct ImagesForm {
    /// The image to process.
    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    file: String,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct ImagesQueryParams {
    /// (Optional) The language to use for the OCR. Defaults to "eng".
    pub language: Option<String>,
    /// (Optional) The model to use for the OCR. Defaults to "eng".
    pub model: Option<String>,
    /// (Optional) Tesseract page segmentation mode. Valid values are 0 through 13.
    #[param(minimum = 0, maximum = 13)]
    pub psm: Option<i32>,
    /// (Optional) Tesseract OCR Engine mode. Valid values are 0 through 3.
    #[param(minimum = 0, maximum = 3)]
    pub oem: Option<i32>,
    /// (Optional) Source resolution in pixels per inch. Must be positive.
    #[param(minimum = 1)]
    pub dpi: Option<i32>,
    /// (Optional) Characters to restrict recognition to.
    pub whitelist: Option<String>,
    /// (Optional) Characters to exclude from recognition.
    pub blacklist: Option<String>,
    /// (Optional) Recognition output format. Defaults to `text`.
    pub format: Option<String>,
}
