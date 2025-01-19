use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

pub mod health;
pub mod images;

use crate::{
    models::{health::HealthResponse, images::ImagesResponse},
    AppState,
};

#[derive(OpenApi)]
#[openapi(components(schemas(ImagesResponse)))]
pub struct ImagesApi;

impl ImagesApi {
    pub fn router() -> OpenApiRouter<AppState> {
        OpenApiRouter::with_openapi(ImagesApi::openapi()).routes(routes!(images::images))
    }
}

#[derive(OpenApi)]
#[openapi(components(schemas(HealthResponse)))]
pub struct HealthApi;

impl HealthApi {
    pub fn router() -> OpenApiRouter<AppState> {
        OpenApiRouter::with_openapi(HealthApi::openapi()).routes(routes!(health::health))
    }
}
