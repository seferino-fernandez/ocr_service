use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[non_exhaustive]
pub struct HealthResponse {
    pub status: String,
}

impl HealthResponse {
    #[must_use]
    pub fn new(status: &str) -> Self {
        Self {
            status: status.to_owned(),
        }
    }
}
