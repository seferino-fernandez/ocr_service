use tower_http::cors::{AllowHeaders, Any, CorsLayer};

use crate::config::app_config::SecurityConfig;

/// Layer to configure CORS / CORS headers.
pub fn cors_layer(security_config: &SecurityConfig) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(AllowHeaders::mirror_request())
        .max_age(security_config.max_access_control_age)
}
