use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[non_exhaustive]
pub struct ImagesResponse {
    /// The text extracted from the image.
    pub text: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[allow(unused)]
#[non_exhaustive]
pub struct ImagesForm {
    /// The image to process.
    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    file: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[non_exhaustive]
pub struct ImagesQueryParams {
    /// (Optional) The language to use for the OCR. Defaults to "eng".
    pub language: Option<String>,
    /// (Optional) The model to use for the OCR. Defaults to "eng".
    pub model: Option<String>,
}
