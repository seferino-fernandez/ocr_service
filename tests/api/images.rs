use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use http_body_util::BodyExt as _;
use tokio::fs::read;

use crate::helpers::*;

#[tokio::test]
async fn test_images_endpoint_tessdoc_introduction() {
    let app = TestApp::new();

    // Read test image file
    let image_data = read("tests/images/tessdoc-introduction.png").await.unwrap();

    // Create multipart form data
    let body = create_multipart_body("image", "tessdoc-introduction.png", &image_data);

    let req = Request::post("/api/v1/images")
        .header(
            CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", BOUNDARY),
        )
        .body(body)
        .unwrap();

    let response = app.request(req).await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    insta::assert_yaml_snapshot!(body);
}

// Helper constant for multipart boundary
const BOUNDARY: &str = "test_boundary";

// Helper function to create multipart form data
fn create_multipart_body(field_name: &str, filename: &str, data: &[u8]) -> Body {
    let mut body = Vec::new();

    // Add multipart boundary and headers
    body.extend_from_slice(format!("--{}\r\n", BOUNDARY).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            field_name, filename
        )
        .as_bytes(),
    );
    body.extend_from_slice("Content-Type: image/png\r\n\r\n".as_bytes());

    // Add file data
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{}--\r\n", BOUNDARY).as_bytes());

    Body::from(body)
}
