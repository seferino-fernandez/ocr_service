use thiserror::Error;

pub type ServerResult<T> = core::result::Result<T, ServerError>;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Failed to load the environnement variable file, not found : `{0}`")]
    ConfigMissingEnv(&'static str),
    #[error("Failed to load the environnement variable file, wrong format : `{0}`")]
    ConfigWrongFormat(&'static str),
}
