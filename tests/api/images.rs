#![expect(
    clippy::unwrap_used,
    reason = "test scaffolding: a failure here means the setup itself is broken, and panicking reports that directly"
)]

use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, Response, StatusCode, header::CONTENT_TYPE},
};
use http_body_util::BodyExt as _;
use ocr_service::{AppState, config::app_config::AppConfig, ocr::OcrEngine};
use tokio::fs::read;

use crate::helpers::*;

/// A one-pixel PNG, so tests can send bytes that really do decode as an image.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xdd, 0x8d, 0xb0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
    0x44, 0xae, 0x42, 0x60, 0x82,
];

/// Builds a POST to the images endpoint with the given query string and body.
fn images_request(query: &str, body: Body) -> Request<Body> {
    Request::post(format!("/api/v1/images{query}"))
        .header(CONTENT_TYPE, multipart_content_type())
        .body(body)
        .unwrap()
}

/// Reads the recognized text from a successful images response.
async fn recognized_text(response: Response<Body>) -> String {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    body.get("text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Runs Recognition for the real document fixture with the given query string.
async fn recognize_tessdoc(query: &str) -> Response<Body> {
    let app = TestApp::new();
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    app.request(images_request(
        query,
        multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
    ))
    .await
}

/// Builds a test app over the real Tessdata Catalog with customized configuration.
fn real_app_with_config(customize: impl FnOnce(&mut AppConfig)) -> (TestApp, Arc<OcrEngine>) {
    let mut app_config = AppConfig::default();
    customize(&mut app_config);
    let state = AppState::from_config(app_config).unwrap();
    let ocr = Arc::clone(&state.ocr);

    (TestApp::with_state(state), ocr)
}

/// Reads a native-format response body as UTF-8 text.
async fn native_output(response: Response<Body>) -> String {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn test_images_endpoint_tessdoc_introduction() {
    let app = TestApp::new();

    // Read test image file
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    let req = images_request(
        "",
        multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
    );

    let response = app.request(req).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "application/json"
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let object = body.as_object().unwrap();
    assert_eq!(object.len(), 2, "got {body}");
    assert!(object.contains_key("text"), "got {body}");
    let confidence = object
        .get("confidence")
        .and_then(serde_json::Value::as_i64)
        .unwrap();
    assert!((0..=100).contains(&confidence), "got {confidence}");
    insta::with_settings!({filters => vec![
        (r"\\n\*", r"\ne"),
    ]}, {
        insta::assert_yaml_snapshot!(body);
    });
}

#[tokio::test]
async fn text_format_returns_json_with_real_confidence() {
    let response = recognize_tessdoc("?format=text").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let object = body.as_object().unwrap();
    assert_eq!(object.len(), 2, "got {body}");
    assert!(
        object
            .get("text")
            .and_then(serde_json::Value::as_str)
            .is_some()
    );
    let confidence = object
        .get("confidence")
        .and_then(serde_json::Value::as_i64)
        .unwrap();
    assert!((0..=100).contains(&confidence), "got {confidence}");
}

#[tokio::test]
async fn hocr_format_returns_native_hocr() {
    let response = recognize_tessdoc("?format=hocr").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "text/vnd.hocr+html"
    );
    let body = native_output(response).await;
    assert!(
        body.contains("class='ocr_page'") || body.contains("ocr_line"),
        "got {body:?}"
    );
}

#[tokio::test]
async fn alto_format_returns_native_alto_xml() {
    let response = recognize_tessdoc("?format=alto").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "application/xml"
    );
    let body = native_output(response).await;
    assert!(body.trim_start().starts_with("<?xml"), "got {body:?}");
    assert!(body.contains("<alto xmlns="), "got {body:?}");
    assert!(body.trim_end().ends_with("</alto>"), "got {body:?}");
}

#[tokio::test]
async fn tsv_format_returns_native_tesseract_tsv() {
    let response = recognize_tessdoc("?format=tsv").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "text/tab-separated-values"
    );
    let body = native_output(response).await;
    let mut lines = body.lines();
    assert_eq!(
        lines.next().unwrap(),
        "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext"
    );
    let first_row = lines.next().unwrap();
    assert_eq!(first_row.split('\t').count(), 12, "got {first_row:?}");
}

#[tokio::test]
async fn box_format_returns_native_tesseract_box_file() {
    let response = recognize_tessdoc("?format=box").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(CONTENT_TYPE).unwrap(), "text/plain");
    let body = native_output(response).await;
    let fields = body
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 6, "got {fields:?}");
    for coordinate in &fields[1..5] {
        coordinate.parse::<i32>().unwrap();
    }
    assert_eq!(fields.last().unwrap(), &"0");
}

#[tokio::test]
async fn an_unknown_output_format_is_rejected() {
    let response = recognize_tessdoc("?format=nonsense").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_message(response).await,
        "Invalid format value 'nonsense': accepted values are text, hocr, alto, tsv, box"
    );
}

#[tokio::test]
async fn a_character_whitelist_restricts_recognition() {
    let app = TestApp::new();
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    let unrestricted = app
        .request(images_request(
            "",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
        .await;
    assert_eq!(unrestricted.status(), StatusCode::OK);
    let unrestricted = recognized_text(unrestricted).await;
    assert!(unrestricted.chars().any(char::is_alphabetic));

    let digits_only = app
        .request(images_request(
            "?whitelist=0123456789",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
        .await;
    assert_eq!(digits_only.status(), StatusCode::OK);
    let digits_only = recognized_text(digits_only).await;
    assert!(
        digits_only
            .chars()
            .any(|character| character.is_ascii_digit())
    );
    assert!(!digits_only.chars().any(char::is_alphabetic));
}

#[tokio::test]
async fn a_character_whitelist_does_not_leak_between_recognitions() {
    let (app, ocr) = real_app_with_config(|app_config| app_config.ocr.pool_size = 1);
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    let digits_only = app
        .request(images_request(
            "?whitelist=0123456789",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
        .await;
    assert_eq!(digits_only.status(), StatusCode::OK);
    let digits_only = recognized_text(digits_only).await;
    assert!(
        digits_only
            .chars()
            .any(|character| character.is_ascii_digit())
    );
    assert!(!digits_only.chars().any(char::is_alphabetic));

    let unrestricted = app
        .request(images_request(
            "",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
        .await;
    assert_eq!(unrestricted.status(), StatusCode::OK);
    let unrestricted = recognized_text(unrestricted).await;
    assert!(
        unrestricted.contains("Tesseract is an open source text recognition"),
        "got {unrestricted:?}"
    );
    assert_eq!(ocr.constructed_engine_count(), 1);
}

#[tokio::test]
async fn page_segmentation_mode_does_not_leak_between_recognitions() {
    let (app, ocr) = real_app_with_config(|app_config| app_config.ocr.pool_size = 1);
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    let single_line = app
        .request(images_request(
            "?psm=7",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
        .await;
    assert_eq!(single_line.status(), StatusCode::OK);
    let single_line = recognized_text(single_line).await;

    let full_page = app
        .request(images_request(
            "",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
        .await;
    assert_eq!(full_page.status(), StatusCode::OK);
    let full_page = recognized_text(full_page).await;
    assert!(
        full_page.contains("Tesseract is an open source text recognition"),
        "got {full_page:?}"
    );
    assert!(
        full_page.lines().count() > single_line.lines().count(),
        "single line: {single_line:?}; full page: {full_page:?}"
    );
    assert_eq!(ocr.constructed_engine_count(), 1);
}

#[tokio::test]
async fn an_explicit_engine_mode_does_not_reconfigure_a_pooled_engine() {
    let (app, ocr) = real_app_with_config(|app_config| app_config.ocr.pool_size = 1);
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    let legacy = app
        .request(images_request(
            "?oem=0",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
        .await;
    assert_eq!(legacy.status(), StatusCode::OK);
    let legacy = recognized_text(legacy).await;

    let default = app
        .request(images_request(
            "",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
        .await;
    assert_eq!(default.status(), StatusCode::OK);
    let default = recognized_text(default).await;

    assert!(!legacy.trim().is_empty(), "got {legacy:?}");
    assert_ne!(legacy, default);
    assert!(
        default.contains("Tesseract is an open source text recognition"),
        "got {default:?}"
    );
    assert_eq!(ocr.constructed_engine_count(), 2);
}

#[tokio::test]
async fn repeated_recognitions_reuse_one_initialized_engine() {
    let (app, ocr) = real_app_with_config(|app_config| app_config.ocr.pool_size = 1);
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    for _request_number in 0..2 {
        let response = app
            .request(images_request(
                "",
                multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
            ))
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    assert_eq!(ocr.constructed_engine_count(), 1);
}

#[tokio::test]
async fn a_zero_pool_size_constructs_an_engine_per_recognition() {
    let (app, ocr) = real_app_with_config(|app_config| app_config.ocr.pool_size = 0);
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    for _request_number in 0..2 {
        let response = app
            .request(images_request(
                "",
                multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
            ))
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    assert_eq!(ocr.constructed_engine_count(), 2);
}

#[tokio::test]
async fn concurrent_recognitions_do_not_share_a_checked_out_engine() {
    let (app, ocr) = real_app_with_config(|app_config| app_config.ocr.pool_size = 2);
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();
    let recognitions = (0..6).map(|_request_number| {
        app.request(images_request(
            "",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
    });

    for response in futures_util::future::join_all(recognitions).await {
        assert_eq!(response.status(), StatusCode::OK);
        let text = recognized_text(response).await;
        assert!(
            text.contains("Tesseract is an open source text recognition"),
            "got {text:?}"
        );
    }
    assert_eq!(ocr.constructed_engine_count(), 2);
}

#[tokio::test]
async fn recognition_timeout_returns_a_clear_error() {
    let (app, _ocr) = real_app_with_config(|app_config| {
        app_config.ocr.recognition_timeout = Duration::ZERO;
    });
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    let response = app
        .request(images_request(
            "",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        error_message(response)
            .await
            .contains("Recognition timed out after 0 seconds")
    );
}

#[tokio::test]
async fn a_character_blacklist_excludes_characters_from_recognition() {
    let app = TestApp::new();
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    let response = app
        .request(images_request(
            "?blacklist=ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            multipart_body("tessdoc-introduction.png", Some("image/png"), &image_data),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let text = recognized_text(response).await;
    assert!(text.chars().any(|character| character.is_ascii_digit()));
    assert!(
        !text
            .chars()
            .any(|character| character.is_ascii_alphabetic()),
        "got {text:?}"
    );
}

#[tokio::test]
async fn unavailable_language_is_rejected() {
    let synthetic = SyntheticApp::new();

    let response = synthetic
        .app
        .request(images_request(
            "?language=xyz",
            multipart_body("image.png", Some("image/png"), TINY_PNG),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_message(response).await,
        "Language 'xyz' is not available"
    );
}

#[tokio::test]
async fn a_language_with_several_models_requires_an_explicit_choice() {
    let synthetic = SyntheticApp::new();

    let response = synthetic
        .app
        .request(images_request(
            "?language=chi_sim",
            multipart_body("image.png", Some("image/png"), TINY_PNG),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_message(response).await,
        "Multiple models available for language 'chi_sim', please specify a model"
    );
}

#[tokio::test]
async fn a_model_without_a_language_is_rejected() {
    let synthetic = SyntheticApp::new();

    let response = synthetic
        .app
        .request(images_request(
            "?model=chi_sim",
            multipart_body("image.png", Some("image/png"), TINY_PNG),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_message(response).await,
        "Language must be specified when model is provided"
    );
}

#[tokio::test]
async fn an_out_of_range_page_segmentation_mode_is_rejected() {
    let synthetic = SyntheticApp::new();

    let response = synthetic
        .app
        .request(images_request(
            "?psm=99",
            multipart_body("image.png", Some("image/png"), TINY_PNG),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_message(response).await,
        "Invalid psm value 99: valid range is 0..=13"
    );
}

#[tokio::test]
async fn an_out_of_range_ocr_engine_mode_is_rejected() {
    let synthetic = SyntheticApp::new();

    let response = synthetic
        .app
        .request(images_request(
            "?oem=9",
            multipart_body("image.png", Some("image/png"), TINY_PNG),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_message(response).await,
        "Invalid oem value 9: valid range is 0..=3"
    );
}

#[tokio::test]
async fn a_non_positive_source_resolution_is_rejected() {
    let synthetic = SyntheticApp::new();

    let response = synthetic
        .app
        .request(images_request(
            "?dpi=0",
            multipart_body("image.png", Some("image/png"), TINY_PNG),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_message(response).await,
        "Invalid dpi value 0: valid range is 1..=2147483647"
    );
}

#[tokio::test]
async fn an_unsupported_content_type_is_rejected() {
    let synthetic = SyntheticApp::new();

    let response = synthetic
        .app
        .request(images_request(
            "",
            multipart_body("notes.txt", Some("text/plain"), b"just some text"),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_message(response).await,
        "Invalid file type: text/plain. File types allowed: \
         image/png,image/jpg,image/jpeg,image/webp,image/gif"
    );
}

#[tokio::test]
async fn a_missing_content_type_is_rejected() {
    let synthetic = SyntheticApp::new();

    let response = synthetic
        .app
        .request(images_request(
            "",
            multipart_body("mystery", None, TINY_PNG),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        error_message(response).await,
        "No content type provided for given file"
    );
}

#[tokio::test]
async fn bytes_that_are_not_an_image_are_rejected() {
    let synthetic = SyntheticApp::new();

    let response = synthetic
        .app
        .request(images_request(
            "",
            multipart_body("lies.png", Some("image/png"), b"this is not a png"),
        ))
        .await;

    // The content type claims PNG, so the failure has to come from decoding.
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_body_over_the_configured_limit_is_rejected() {
    // Guards the layer ordering fixed in 5f53e73: DefaultBodyLimit is disabled so that
    // RequestBodyLimitLayer is the one enforcing this.
    let synthetic = SyntheticApp::with_config(|app_config| {
        app_config.server.file_upload_max_size = 64;
        app_config.server.file_upload_max_size_enabled = true;
    });

    let oversized = vec![0_u8; 4096];
    let response = synthetic
        .app
        .request(images_request(
            "",
            multipart_body("big.png", Some("image/png"), &oversized),
        ))
        .await;

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn a_body_within_the_configured_limit_is_accepted() {
    let synthetic = SyntheticApp::with_config(|app_config| {
        app_config.server.file_upload_max_size = 64 * 1024;
        app_config.server.file_upload_max_size_enabled = true;
    });

    let response = synthetic
        .app
        .request(images_request(
            "?language=xyz",
            multipart_body("small.png", Some("image/png"), TINY_PNG),
        ))
        .await;

    // Rejected for the language, not the size — the limit did not fire.
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
