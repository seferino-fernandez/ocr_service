use axum::{
    Json,
    extract::multipart::MultipartError,
    extract::rejection::JsonRejection,
    response::{IntoResponse, Response},
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// Custom error type for the API.
/// The `#[from]` attribute allows for easy conversion from other error types.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ErrorType {
    /// Converts from an Axum built-in extractor error.
    #[error("Invalid payload.")]
    InvalidJsonBody(#[from] JsonRejection),

    /// Converts from Axum's multipart extractor.
    ///
    /// Carried whole rather than flattened to a string, so the extractor's own status
    /// survives — an upload over the configured limit has to stay a 413, not become a 400.
    #[error("Invalid multipart payload.")]
    Multipart(#[from] MultipartError),

    /// For errors that occur during manual validation.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Converts from any `anyhow::Error`.
    #[error("An internal server error has occurred.")]
    InternalError(#[from] anyhow::Error),
}

/// Adapts an OCR failure onto the HTTP surface.
///
/// A caller's bad upload is a 400; a Tesseract failure is a 500. Keeping the mapping
/// here is what lets [`crate::ocr`] stay free of any HTTP knowledge.
impl From<crate::ocr::OcrError> for ErrorType {
    fn from(error: crate::ocr::OcrError) -> Self {
        let message = error.to_string();
        match error {
            crate::ocr::OcrError::Engine { .. } => Self::InternalError(anyhow::anyhow!(message)),
            _ => Self::InvalidRequest(message),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct ErrorResponse {
    pub message: String,
}

// The IntoResponse implementation for ApiError logs the error message.
//
// To avoid exposing implementation details to API consumers, we separate
// the message that we log from the API response message.
impl IntoResponse for ErrorType {
    fn into_response(self) -> Response {
        // Log detailed error for telemetry.
        let (error, status) = match self {
            Self::InvalidJsonBody(err) => (
                match err {
                    JsonRejection::JsonDataError(_) => "Invalid JSON data".to_owned(),
                    JsonRejection::JsonSyntaxError(_) => "Invalid JSON syntax".to_owned(),
                    JsonRejection::MissingJsonContentType(_) => {
                        "Missing 'Content-Type: application/json' header".to_owned()
                    }
                    JsonRejection::BytesRejection(_) => "Failed to buffer request body".to_owned(),
                    _ => "Unknown error".to_owned(),
                },
                StatusCode::BAD_REQUEST,
            ),
            // Keep the extractor's own status: an oversized upload is a 413.
            Self::Multipart(err) => (err.body_text(), err.status()),
            Self::InvalidRequest(err) => (err, StatusCode::BAD_REQUEST),
            Self::InternalError(err) => (err.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
        };

        // Log detailed error for telemetry.
        tracing::error!("{}", error);
        // Create a generic response to hide specific implementation details.
        let error_response = ErrorResponse { message: error };

        (status, Json(error_response)).into_response()
    }
}
