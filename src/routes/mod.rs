use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

pub mod health;

pub use health::HealthApi;

pub mod images;
pub mod languages;

use crate::{
    AppState,
    models::{images::ImagesResponse, languages::LanguagesResponse},
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
#[openapi(components(schemas(LanguagesResponse)))]
pub struct LanguagesApi;

impl LanguagesApi {
    pub fn router() -> OpenApiRouter<AppState> {
        OpenApiRouter::with_openapi(LanguagesApi::openapi()).routes(routes!(languages::languages))
    }
}
