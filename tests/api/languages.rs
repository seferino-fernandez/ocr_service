use std::fs;

use axum::{
    body::Body,
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use http_body_util::BodyExt as _;

use crate::helpers::*;

/// A one-pixel PNG, so the request reaches model resolution before Tesseract sees the
/// synthetic model bytes.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xdd, 0x8d, 0xb0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
    0x44, 0xae, 0x42, 0x60, 0x82,
];

#[tokio::test]
async fn test_languages_ok() {
    let app = TestApp::new();

    let req = Request::get("/api/v1/languages")
        .body(Body::empty())
        .unwrap();
    let response = app.request(req).await;
    let headers = response.headers().clone();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(headers.get("access-control-allow-origin").unwrap(), "*");
    assert!(headers.get("vary").is_some());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    insta::assert_yaml_snapshot!(body);
}

#[tokio::test]
async fn language_default_from_models_toml_reaches_ocr_instead_of_returning_bad_request() {
    let synthetic = SyntheticApp::with_tessdata(|tessdata, _app_config| {
        let metadata = r#"
default = "chi_sim_vert"

[models.chi_sim]
description = "Simplified Chinese"

[models.chi_sim_vert]
description = "Vertical Simplified Chinese"
"#;
        fs::write(tessdata.join("chi_sim").join("models.toml"), metadata).unwrap();
    });

    let request = Request::post("/api/v1/images?language=chi_sim")
        .header(CONTENT_TYPE, multipart_content_type())
        .body(multipart_body("image.png", Some("image/png"), TINY_PNG))
        .unwrap();

    let response = synthetic.app.request(request).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
