use axum::{body::Body, http::Request, http::StatusCode};
use http_body_util::BodyExt as _;

use crate::helpers::*;

#[tokio::test]
async fn test_languages_ok() {
    let app = TestApp::new();

    let req = Request::get("/api/languages").body(Body::empty()).unwrap();
    let response = app.request(req).await;
    let headers = response.headers().clone();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(headers.get("access-control-allow-origin").unwrap(), "*");
    assert!(headers.get("vary").is_some());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    insta::assert_yaml_snapshot!(body);
}
