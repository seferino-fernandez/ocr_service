use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt as _;
use ocr_service::ocr::{OcrEngine, OcrError};

use crate::helpers::*;

#[tokio::test]
async fn config_returns_the_live_client_visible_settings() {
    const UPLOAD_LIMIT: usize = 37_422_001;

    let synthetic = SyntheticApp::with_config(|app_config| {
        app_config.server.file_upload_max_size = UPLOAD_LIMIT;
        app_config.server.file_upload_max_size_enabled = false;
        app_config.server.timeout = Duration::from_secs(42);
        app_config.service.default_language = "deu".to_owned();
    });

    let request = Request::get("/system/config").body(Body::empty()).unwrap();
    let response = synthetic.app.request(request).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "file_upload_max_size": UPLOAD_LIMIT,
            "file_upload_max_size_enabled": false,
            "request_timeout_seconds": 42,
            "default_language": "deu",
            "allowed_file_types": ocr_allowed_file_types(),
        })
    );
}

#[tokio::test]
async fn config_does_not_expose_secret_bearing_keys() {
    let synthetic = SyntheticApp::with_config(|app_config| {
        app_config.otel_provider.auth_token = Some("must-not-be-exposed".to_owned());
        app_config.otel_provider.organization = Some("private-organization".to_owned());
        app_config.otel_provider.stream_name = Some("private-stream".to_owned());
    });

    let request = Request::get("/system/config").body(Body::empty()).unwrap();
    let response = synthetic.app.request(request).await;
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(!contains_secret_bearing_key(&body));
}

fn ocr_allowed_file_types() -> Vec<String> {
    match OcrEngine::check_content_type("") {
        Err(OcrError::UnsupportedContentType { allowed, .. }) => {
            allowed.split(',').map(str::to_owned).collect()
        }
        Err(_) | Ok(()) => Vec::new(),
    }
}

fn contains_secret_bearing_key(value: &serde_json::Value) -> bool {
    const SECRET_BEARING_KEYS: [&str; 4] =
        ["auth_token", "organization", "stream_name", "otel_provider"];

    match value {
        serde_json::Value::Array(values) => values.iter().any(contains_secret_bearing_key),
        serde_json::Value::Object(values) => values.iter().any(|(key, value)| {
            SECRET_BEARING_KEYS.contains(&key.as_str()) || contains_secret_bearing_key(value)
        }),
        _ => false,
    }
}
