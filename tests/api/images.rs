#![expect(
    clippy::unwrap_used,
    reason = "test scaffolding: a failure here means the setup itself is broken, and panicking reports that directly"
)]

use axum::{
    body::Body,
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use http_body_util::BodyExt as _;
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

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    insta::with_settings!({filters => vec![
        (r"\\n\*", r"\ne"),
    ]}, {
        insta::assert_yaml_snapshot!(body);
    });
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
