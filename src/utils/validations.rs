use std::collections::HashSet;

use crate::models::{error::ErrorType, images::ImagesQueryParams, languages::TesseractModel};

/// Allowed file types
const ALLOWED_FILE_TYPES: [&str; 5] = [
    "image/png",
    "image/jpg",
    "image/jpeg",
    "image/webp",
    "image/gif",
];

pub fn validate_language_params(
    language_params: &ImagesQueryParams,
    available_languages: &HashSet<TesseractModel>,
    default_language: &str,
) -> Result<TesseractModel, ErrorType> {
    // If model is provided, language must also be provided
    if language_params.model.is_some() && language_params.language.is_none() {
        return Err(ErrorType::InvalidRequest(
            "Language must be specified when model is provided".to_owned(),
        ));
    }

    // Use the provided language or default to the configured default language
    let language = language_params
        .language
        .as_deref()
        .unwrap_or(default_language);

    // Filter models that match the requested language
    let matching_language_models: Vec<&TesseractModel> = available_languages
        .iter()
        .filter(|model| model.language == language)
        .collect();

    // If no models match the requested language
    if matching_language_models.is_empty() {
        return Err(ErrorType::InvalidRequest(format!(
            "Language '{}' is not available",
            language
        )));
    }

    // If a specific model is requested
    if let Some(requested_model) = &language_params.model {
        // Find model that matches both language and model name
        if let Some(model) = matching_language_models
            .iter()
            .find(|m| m.model.as_deref() == Some(requested_model))
        {
            return Ok((*model).clone());
        }

        // No matching model found for the requested language and model
        return Err(ErrorType::InvalidRequest(format!(
            "Model '{}' not found for language '{}'",
            requested_model, language
        )));
    }

    // If only language is provided (no specific model)
    if matching_language_models.len() == 1 {
        // If there's only one model for this language, use it
        Ok(matching_language_models[0].clone())
    } else {
        // If there are multiple models for this language and no specific model was requested
        // Find a model without a specific model name if possible
        if let Some(model) = matching_language_models.iter().find(|m| m.model.is_none()) {
            return Ok((*model).clone());
        }

        // If all models have a specific model name, we need the user to specify which one
        Err(ErrorType::InvalidRequest(format!(
            "Multiple models available for language '{}', please specify a model",
            language
        )))
    }
}

/// Validate the file type
///
/// # Errors
///
/// Returns an error if the file type is not allowed
pub fn validate_file_type(file_type: &str) -> Result<(), ErrorType> {
    if !ALLOWED_FILE_TYPES.contains(&file_type) {
        return Err(ErrorType::InvalidRequest(format!(
            "Invalid file type: {}. File types allowed: {}",
            file_type,
            ALLOWED_FILE_TYPES.join(",")
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        models::{error::ErrorType, images::ImagesQueryParams, languages::TesseractModel},
        utils::validations::{validate_file_type, validate_language_params},
    };
    use std::collections::HashSet;

    #[test]
    fn test_validate_file_type_valid() {
        assert!(validate_file_type("image/png").is_ok());
        assert!(validate_file_type("image/jpg").is_ok());
        assert!(validate_file_type("image/jpeg").is_ok());
        assert!(validate_file_type("image/webp").is_ok());
        assert!(validate_file_type("image/gif").is_ok());
    }

    #[test]
    fn test_validate_file_type_invalid() {
        let result = validate_file_type("text/plain");
        assert!(result.is_err());
        match result {
            Err(ErrorType::InvalidRequest(msg)) => {
                assert_eq!(msg, "Invalid file type");
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_file_type_empty() {
        let result = validate_file_type("");
        assert!(result.is_err());
        match result {
            Err(ErrorType::InvalidRequest(msg)) => {
                assert_eq!(msg, "Invalid file type");
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_language_params_model_without_language() {
        let params = ImagesQueryParams {
            language: None,
            model: Some("fast".to_string()),
        };
        let available_languages = HashSet::new();

        let result = validate_language_params(&params, &available_languages, "eng");
        assert!(result.is_err());
        match result {
            Err(ErrorType::InvalidRequest(msg)) => {
                assert_eq!(msg, "Language must be specified when model is provided");
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_language_params_unavailable_language() {
        let params = ImagesQueryParams {
            language: Some("xyz".to_string()),
            model: None,
        };
        let available_languages = HashSet::new();

        let result = validate_language_params(&params, &available_languages, "eng");
        assert!(result.is_err());
        match result {
            Err(ErrorType::InvalidRequest(msg)) => {
                assert_eq!(msg, "Language 'xyz' is not available");
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_language_params_language_and_model_match() {
        let mut available_languages = HashSet::new();
        available_languages.insert(TesseractModel {
            language: "spa".to_string(),
            model: Some("fast".to_string()),
            full_path: Some("spa/spa_fast.traineddata".to_string()),
            relative_path: Some("spa/spa_fast".to_string()),
        });
        available_languages.insert(TesseractModel {
            language: "spa".to_string(),
            model: Some("default".to_string()),
            full_path: Some("spa/spa_default.traineddata".to_string()),
            relative_path: Some("spa/spa_default".to_string()),
        });

        let params = ImagesQueryParams {
            language: Some("spa".to_string()),
            model: Some("fast".to_string()),
        };

        let result = validate_language_params(&params, &available_languages, "eng");
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.language, "spa");
        assert_eq!(model.model, Some("fast".to_string()));
    }

    #[test]
    fn test_validate_language_params_model_not_found_for_language() {
        let mut available_languages = HashSet::new();
        available_languages.insert(TesseractModel {
            language: "spa".to_string(),
            model: Some("fast".to_string()),
            full_path: Some("spa/spa_fast.traineddata".to_string()),
            relative_path: Some("spa/spa_fast".to_string()),
        });

        let params = ImagesQueryParams {
            language: Some("spa".to_string()),
            model: Some("slow".to_string()),
        };

        let result = validate_language_params(&params, &available_languages, "eng");
        assert!(result.is_err());
        match result {
            Err(ErrorType::InvalidRequest(msg)) => {
                assert_eq!(msg, "Model 'slow' not found for language 'spa'");
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_language_params_only_language_one_model() {
        let mut available_languages = HashSet::new();
        available_languages.insert(TesseractModel {
            language: "spa".to_string(),
            model: Some("fast".to_string()),
            full_path: Some("spa/spa_fast.traineddata".to_string()),
            relative_path: Some("spa/spa_fast".to_string()),
        });

        let params = ImagesQueryParams {
            language: Some("spa".to_string()),
            model: None,
        };

        let result = validate_language_params(&params, &available_languages, "eng");
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.language, "spa");
        assert_eq!(model.model, Some("fast".to_string()));
    }

    #[test]
    fn test_validate_language_params_multiple_models_one_default() {
        let mut available_languages = HashSet::new();
        available_languages.insert(TesseractModel {
            language: "eng".to_string(),
            model: None,
            full_path: Some("eng.traineddata".to_string()),
            relative_path: Some("eng".to_string()),
        });
        available_languages.insert(TesseractModel {
            language: "eng".to_string(),
            model: Some("fast".to_string()),
            full_path: Some("eng/eng_fast.traineddata".to_string()),
            relative_path: Some("eng/eng_fast".to_string()),
        });

        let params = ImagesQueryParams {
            language: Some("eng".to_string()),
            model: None,
        };

        let result = validate_language_params(&params, &available_languages, "eng");
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.language, "eng");
        assert_eq!(model.model, None);
    }

    #[test]
    fn test_validate_language_params_multiple_models_no_default() {
        let mut available_languages = HashSet::new();
        available_languages.insert(TesseractModel {
            language: "eng".to_string(),
            model: Some("fast".to_string()),
            full_path: Some("eng/eng_fast.traineddata".to_string()),
            relative_path: Some("eng/eng_fast".to_string()),
        });
        available_languages.insert(TesseractModel {
            language: "eng".to_string(),
            model: Some("best".to_string()),
            full_path: Some("eng/eng_best.traineddata".to_string()),
            relative_path: Some("eng/eng_best".to_string()),
        });

        let params = ImagesQueryParams {
            language: Some("eng".to_string()),
            model: None,
        };

        let result = validate_language_params(&params, &available_languages, "eng");
        assert!(result.is_err());
        match result {
            Err(ErrorType::InvalidRequest(msg)) => {
                assert_eq!(
                    msg,
                    "Multiple models available for language 'eng', please specify a model"
                );
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_language_params_use_default_language() {
        let mut available_languages = HashSet::new();
        available_languages.insert(TesseractModel {
            language: "eng".to_string(),
            model: None,
            full_path: Some("eng.traineddata".to_string()),
            relative_path: Some("eng".to_string()),
        });

        let params = ImagesQueryParams {
            language: None,
            model: None,
        };

        let result = validate_language_params(&params, &available_languages, "eng");
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.language, "eng");
        assert_eq!(model.model, None);
    }

    #[test]
    fn test_validate_language_params_default_language_not_available() {
        let mut available_languages = HashSet::new();
        available_languages.insert(TesseractModel {
            language: "spa".to_string(),
            model: None,
            full_path: Some("spa.traineddata".to_string()),
            relative_path: Some("spa".to_string()),
        });

        let params = ImagesQueryParams {
            language: None,
            model: None,
        };

        let result = validate_language_params(&params, &available_languages, "eng");
        assert!(result.is_err());
        match result {
            Err(ErrorType::InvalidRequest(msg)) => {
                assert_eq!(msg, "Language 'eng' is not available");
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }
}
