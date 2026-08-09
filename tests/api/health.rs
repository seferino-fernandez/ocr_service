use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt as _;

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

#[tokio::test]
async fn health_trailing_slash_matches_the_canonical_route() {
    let app = TestApp::new();

    let canonical = app
        .request(Request::get("/system/health").body(Body::empty()).unwrap())
        .await;
    let trailing_slash = app
        .request(Request::get("/system/health/").body(Body::empty()).unwrap())
        .await;

    assert_eq!(canonical.status(), StatusCode::OK);
    assert_eq!(trailing_slash.status(), StatusCode::OK);
    assert_eq!(
        canonical.into_body().collect().await.unwrap().to_bytes(),
        trailing_slash
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
    );
}
