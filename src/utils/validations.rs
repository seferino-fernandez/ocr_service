use crate::models::error::ErrorType;

const ALLOWED_LANGUAGES: [&str; 1] = ["eng"];

/// Allowed file types
const ALLOWED_FILE_TYPES: [&str; 5] = [
    "image/png",
    "image/jpg",
    "image/jpeg",
    "image/webp",
    "image/gif",
];

/// Validate the language
///
/// # Errors
///
/// Returns an error if the language is not allowed
pub fn validate_language(language: &str) -> Result<(), ErrorType> {
    if !ALLOWED_LANGUAGES.contains(&language) {
        return Err(ErrorType::InvalidRequest("Invalid language".to_owned()));
    }
    Ok(())
}

/// Validate the file type
///
/// # Errors
///
/// Returns an error if the file type is not allowed
pub fn validate_file_type(file_type: &str) -> Result<(), ErrorType> {
    if !ALLOWED_FILE_TYPES.contains(&file_type) {
        return Err(ErrorType::InvalidRequest("Invalid file type".to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        models::error::ErrorType,
        utils::validations::{validate_file_type, validate_language},
    };

    #[test]
    fn test_validate_language_valid() {
        assert!(validate_language("eng").is_ok());
    }

    #[test]
    fn test_validate_language_invalid() {
        let result = validate_language("fra");
        assert!(result.is_err());
        match result {
            Err(ErrorType::InvalidRequest(msg)) => {
                assert_eq!(msg, "Invalid language");
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_validate_language_empty() {
        let result = validate_language("");
        assert!(result.is_err());
        match result {
            Err(ErrorType::InvalidRequest(msg)) => {
                assert_eq!(msg, "Invalid language");
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

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
}
