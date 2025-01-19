use axum::{
    body::Body,
    http::{Request, StatusCode},
};

use crate::helpers::*;

#[tokio::test]
async fn test_health_check_ok() {
    let app = TestApp::new();

    let req = Request::get("/system/health").body(Body::empty()).unwrap();
    let resp = app.request(req).await;
    let headers = resp.headers().clone();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(headers.get("access-control-allow-origin").unwrap(), "*");
    assert!(headers.get("vary").is_some());
}
