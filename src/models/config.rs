use serde::Serialize;
use utoipa::ToSchema;

/// The upload and recognition settings clients need before making a request.
#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct ConfigResponse {
    /// Maximum upload size in bytes.
    pub file_upload_max_size: usize,
    /// Whether the maximum upload size is currently enforced.
    pub file_upload_max_size_enabled: bool,
    /// Maximum duration of a request in seconds.
    pub request_timeout_seconds: u64,
    /// Language selected when an OCR request does not name one.
    pub default_language: String,
    /// Content types the OCR Engine accepts.
    pub allowed_file_types: Vec<String>,
}

impl ConfigResponse {
    #[must_use]
    pub fn new(
        file_upload_max_size: usize,
        file_upload_max_size_enabled: bool,
        request_timeout_seconds: u64,
        default_language: String,
        allowed_file_types: Vec<String>,
    ) -> Self {
        Self {
            file_upload_max_size,
            file_upload_max_size_enabled,
            request_timeout_seconds,
            default_language,
            allowed_file_types,
        }
    }
}
