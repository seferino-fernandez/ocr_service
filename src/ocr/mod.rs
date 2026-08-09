//! The OCR Engine: turns image bytes into text using a Tesseract Model.
//!
//! Everything Tesseract-specific lives behind this module's interface — image decoding,
//! the pixel layout the C API expects, and the FFI call sequence. Callers supply bytes
//! and a model and receive text, with no knowledge of how recognition happens.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use axum::body::Bytes;
use image::ImageReader;
use tesseract_rs::{TessPageSegMode, TesseractAPI};
use thiserror::Error;

use crate::config::app_config::TesseractConfig;
use crate::tessdata::TesseractModel;

/// Content types the engine can decode.
const ALLOWED_FILE_TYPES: [&str; 5] = [
    "image/png",
    "image/jpg",
    "image/jpeg",
    "image/webp",
    "image/gif",
];

/// Tesseract is handed RGB8, which is three bytes per pixel.
const BYTES_PER_PIXEL: u32 = 3;

/// [`BYTES_PER_PIXEL`] in the signed width Tesseract's C API takes. Must match.
const BYTES_PER_PIXEL_C_INT: i32 = 3;

/// Everything that can go wrong turning an upload into text.
///
/// The variants distinguish a caller's mistake from an engine failure; mapping them onto
/// HTTP lives in `From<OcrError> for ErrorType`, keeping this module free of HTTP.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OcrError {
    /// A recognition option is outside the range Tesseract accepts.
    #[error("Invalid {parameter} value {value}: valid range is {valid_range}")]
    InvalidRecognitionOption {
        /// The query parameter being validated.
        parameter: &'static str,
        /// The rejected value.
        value: i32,
        /// The accepted range.
        valid_range: &'static str,
    },

    /// The upload's content type is not an image format the engine accepts.
    #[error("Invalid file type: {file_type}. File types allowed: {allowed}")]
    UnsupportedContentType {
        /// The rejected content type.
        file_type: String,
        /// The accepted content types, comma separated.
        allowed: String,
    },

    /// The bytes could not be read as an image.
    #[error("{0}")]
    Decode(String),

    /// A dimension does not fit the signed integers Tesseract's C API uses.
    #[error("{0}")]
    Dimensions(String),

    /// Tesseract failed to initialise, or refused the image.
    #[error("Something went wrong while {stage}: {detail}")]
    Engine {
        /// What the engine was doing, phrased to read inside the message.
        stage: &'static str,
        /// The underlying Tesseract error.
        detail: String,
    },

    /// Tesseract could not extract text from the image.
    #[error("Something went wrong while extracting the text: {0}")]
    TextExtraction(String),
}

/// Per-request tuning for one Recognition.
///
/// The default leaves Tesseract's existing configuration untouched.
#[derive(Clone, Debug, Default)]
pub struct RecognitionOptions {
    page_seg_mode: Option<TessPageSegMode>,
    ocr_engine_mode: Option<i32>,
    source_resolution: Option<i32>,
    whitelist: Option<String>,
    blacklist: Option<String>,
}

impl RecognitionOptions {
    /// Validates and constructs per-request Recognition tuning.
    ///
    /// # Errors
    ///
    /// Returns [`OcrError::InvalidRecognitionOption`] when `psm` or `oem` is outside its
    /// accepted range, or when `dpi` is not positive.
    pub fn try_new(
        psm: Option<i32>,
        oem: Option<i32>,
        dpi: Option<i32>,
        whitelist: Option<String>,
        blacklist: Option<String>,
    ) -> Result<Self, OcrError> {
        let page_seg_mode = psm
            .map(|value| {
                if (0..=13).contains(&value) {
                    Ok(TessPageSegMode::from_int(value))
                } else {
                    Err(OcrError::InvalidRecognitionOption {
                        parameter: "psm",
                        value,
                        valid_range: "0..=13",
                    })
                }
            })
            .transpose()?;
        let ocr_engine_mode = validate_range("oem", oem, 0..=3, "0..=3")?;
        let source_resolution = validate_range("dpi", dpi, 1..=i32::MAX, "1..=2147483647")?;

        Ok(Self {
            page_seg_mode,
            ocr_engine_mode,
            source_resolution,
            whitelist,
            blacklist,
        })
    }
}

fn validate_range(
    parameter: &'static str,
    value: Option<i32>,
    valid_values: std::ops::RangeInclusive<i32>,
    valid_range: &'static str,
) -> Result<Option<i32>, OcrError> {
    value
        .map(|value| {
            if valid_values.contains(&value) {
                Ok(value)
            } else {
                Err(OcrError::InvalidRecognitionOption {
                    parameter,
                    value,
                    valid_range,
                })
            }
        })
        .transpose()
}

/// Performs OCR against the Tesseract models in a configured data directory.
#[derive(Debug)]
pub struct OcrEngine {
    data_path: PathBuf,
}

impl OcrEngine {
    /// Creates an engine reading models from the configured tessdata directory.
    #[must_use]
    pub fn new(tesseract: &TesseractConfig) -> Self {
        Self {
            data_path: PathBuf::from(&tesseract.data_path),
        }
    }

    /// Checks that an upload's content type is one the engine can decode.
    ///
    /// Cheap enough to run before buffering the upload's bytes.
    ///
    /// # Errors
    ///
    /// Returns [`OcrError::UnsupportedContentType`] for anything outside the allowed list.
    pub fn check_content_type(content_type: &str) -> Result<(), OcrError> {
        if ALLOWED_FILE_TYPES.contains(&content_type) {
            return Ok(());
        }

        Err(OcrError::UnsupportedContentType {
            file_type: content_type.to_owned(),
            allowed: ALLOWED_FILE_TYPES.join(","),
        })
    }

    /// Extracts the text in `image_bytes` using `model`.
    ///
    /// Decoding and the Tesseract FFI calls are blocking and CPU-bound, so they run on
    /// the blocking pool instead of occupying an async worker thread.
    ///
    /// # Errors
    ///
    /// Returns [`OcrError`] if the bytes are not a decodable image, a dimension does not
    /// fit Tesseract's C API, or Tesseract itself fails.
    pub async fn recognize(
        &self,
        image_bytes: Bytes,
        model: &TesseractModel,
        options: RecognitionOptions,
    ) -> Result<String, OcrError> {
        let data_path = self.data_path.clone();
        let relative_path = model.relative_path.clone();

        tracing::debug!(
            "Running OCR with data path '{}' and model '{relative_path}'",
            data_path.display()
        );

        tokio::task::spawn_blocking(move || {
            recognize_blocking(&data_path, &relative_path, &image_bytes, &options)
        })
        .await
        .map_err(|join_error| OcrError::Engine {
            stage: "performing OCR",
            detail: join_error.to_string(),
        })?
    }
}

/// The blocking half of [`OcrEngine::recognize`].
///
/// The Tesseract handle is created here rather than being carried across the await, so
/// nothing but plain data crosses onto the blocking pool.
fn recognize_blocking(
    data_path: &Path,
    relative_path: &str,
    image_bytes: &[u8],
    options: &RecognitionOptions,
) -> Result<String, OcrError> {
    let image = ImageReader::new(Cursor::new(image_bytes))
        .with_guessed_format()
        .map_err(|error| OcrError::Decode(error.to_string()))?
        .decode()
        .map_err(|error| OcrError::Decode(error.to_string()))?;

    // Tesseract takes raw RGB8 plus the row stride in bytes.
    let rgb_image = image.to_rgb8();
    let (width, height) = rgb_image.dimensions();

    let bytes_per_line: i32 = width
        .checked_mul(BYTES_PER_PIXEL)
        .ok_or_else(|| OcrError::Dimensions("Image dimensions are too large".to_owned()))?
        .try_into()
        .map_err(|error| {
            OcrError::Dimensions(format!("Image dimensions are too large: {error}"))
        })?;
    let width: i32 = width
        .try_into()
        .map_err(|error| OcrError::Dimensions(format!("Image width is too large: {error}")))?;
    let height: i32 = height
        .try_into()
        .map_err(|error| OcrError::Dimensions(format!("Image height is too large: {error}")))?;

    let raw_image_data = rgb_image.into_raw();

    let api = TesseractAPI::new();
    if let Some(ocr_engine_mode) = options.ocr_engine_mode {
        let data_path = data_path.to_str().ok_or_else(|| OcrError::Engine {
            stage: "performing OCR",
            detail: "the tessdata path is not valid UTF-8".to_owned(),
        })?;
        api.init_2(data_path, relative_path, ocr_engine_mode)
            .map_err(|error| OcrError::Engine {
                stage: "performing OCR",
                detail: error.to_string(),
            })?;
    } else {
        api.init(data_path, relative_path)
            .map_err(|error| OcrError::Engine {
                stage: "performing OCR",
                detail: error.to_string(),
            })?;
    }

    if let Some(page_seg_mode) = options.page_seg_mode {
        api.set_page_seg_mode(page_seg_mode)
            .map_err(|error| OcrError::Engine {
                stage: "setting the page segmentation mode",
                detail: error.to_string(),
            })?;
    }

    if let Some(whitelist) = &options.whitelist {
        api.set_variable("tessedit_char_whitelist", whitelist)
            .map_err(|error| OcrError::Engine {
                stage: "setting the character whitelist",
                detail: error.to_string(),
            })?;
    }

    if let Some(blacklist) = &options.blacklist {
        api.set_variable("tessedit_char_blacklist", blacklist)
            .map_err(|error| OcrError::Engine {
                stage: "setting the character blacklist",
                detail: error.to_string(),
            })?;
    }

    api.set_image(
        &raw_image_data,
        width,
        height,
        BYTES_PER_PIXEL_C_INT,
        bytes_per_line,
    )
    .map_err(|error| OcrError::Engine {
        stage: "processing the image",
        detail: error.to_string(),
    })?;

    if let Some(source_resolution) = options.source_resolution {
        api.set_source_resolution(source_resolution)
            .map_err(|error| OcrError::Engine {
                stage: "setting the source resolution",
                detail: error.to_string(),
            })?;
    }

    api.get_utf8_text()
        .map_err(|error| OcrError::TextExtraction(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{ALLOWED_FILE_TYPES, OcrEngine, OcrError, RecognitionOptions};

    #[test]
    fn every_allowed_content_type_is_accepted() {
        for content_type in ALLOWED_FILE_TYPES {
            OcrEngine::check_content_type(content_type)
                .unwrap_or_else(|_ignored| panic!("{content_type} should be accepted"));
        }
    }

    #[test]
    fn a_non_image_content_type_is_rejected() {
        let error =
            OcrEngine::check_content_type("text/plain").expect_err("text/plain should be rejected");

        assert!(matches!(error, OcrError::UnsupportedContentType { .. }));
        assert_eq!(
            error.to_string(),
            "Invalid file type: text/plain. File types allowed: \
             image/png,image/jpg,image/jpeg,image/webp,image/gif"
        );
    }

    #[test]
    fn an_empty_content_type_is_rejected() {
        let error = OcrEngine::check_content_type("")
            .expect_err("an empty content type should be rejected");

        assert_eq!(
            error.to_string(),
            "Invalid file type: . File types allowed: \
             image/png,image/jpg,image/jpeg,image/webp,image/gif"
        );
    }

    #[tokio::test]
    async fn bytes_that_are_not_an_image_fail_to_decode() {
        let engine = OcrEngine::new(&crate::config::app_config::TesseractConfig::default());
        let model = crate::tessdata::TesseractModel {
            language: "eng".to_owned(),
            model: None,
            full_path: "tesseract/eng.traineddata".into(),
            relative_path: "eng".to_owned(),
        };

        let error = engine
            .recognize(
                axum::body::Bytes::from_static(b"not an image"),
                &model,
                RecognitionOptions::default(),
            )
            .await
            .expect_err("expected a decode failure");

        assert!(matches!(error, OcrError::Decode(_)), "got {error:?}");
    }
}
