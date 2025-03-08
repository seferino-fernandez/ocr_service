use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct LanguagesResponse {
    pub languages: Vec<TesseractModel>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TesseractModel {
    /// The language of the Tesseract model.
    pub language: String,
    /// The unique model of the Tesseract language model. Optional if there is only one model for the language.
    pub model: Option<String>,
    /// The full file path of the Tesseract language model.
    pub full_path: Option<String>,
    /// The relative file path of the Tesseract language model without the $TESSDATA_PREFIX or the .traineddata extension.
    pub relative_path: Option<String>,
}
