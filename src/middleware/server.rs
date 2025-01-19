use tower_http::{
    limit::RequestBodyLimitLayer, normalize_path::NormalizePathLayer, timeout::TimeoutLayer,
};

use crate::config::app_config::ServerConfig;

/// Layer to configure the maximum body size for requests.
pub fn body_limit_layer(server_config: &ServerConfig) -> RequestBodyLimitLayer {
    RequestBodyLimitLayer::new(server_config.file_upload_max_size)
}

/// Layer to configure the timeout for requests.
pub fn timeout_layer(server_config: &ServerConfig) -> TimeoutLayer {
    TimeoutLayer::new(server_config.timeout)
}

/// Layer to normalize URL paths.
///
/// Any trailing slashes from request paths will be removed. For example, a request with `/foo/`
/// will be changed to `/foo` before reaching the inner service.
pub fn normalize_path_layer() -> NormalizePathLayer {
    NormalizePathLayer::trim_trailing_slash()
}
