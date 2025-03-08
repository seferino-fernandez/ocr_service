use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

pub mod health;
pub mod images;
pub mod languages;

use crate::{
    models::{health::HealthResponse, images::ImagesResponse, languages::LanguagesResponse},
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

#[derive(OpenApi)]
#[openapi(components(schemas(LanguagesResponse)))]
pub struct LanguagesApi;

impl LanguagesApi {
    pub fn router() -> OpenApiRouter<AppState> {
        OpenApiRouter::with_openapi(LanguagesApi::openapi()).routes(routes!(languages::languages))
    }
}
