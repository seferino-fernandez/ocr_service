use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct LanguagesResponse {
    pub languages: Vec<LanguageSummary>,
}

/// A Tesseract model as described to API consumers.
///
/// Deliberately carries no filesystem detail: the catalog's on-disk representation is
/// [`crate::tessdata::TesseractModel`], which is free to change without breaking the
/// API contract.
///
/// Field order defines the sort order — language first, then model.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct LanguageSummary {
    /// The language of the Tesseract model.
    pub language: String,
    /// The unique model of the Tesseract language model. Absent if there is only one
    /// model for the language.
    pub model: Option<String>,
}
