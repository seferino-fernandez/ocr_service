//! The Tessdata Catalog: the Tesseract models available on disk, and the authority on
//! which one answers a given request.
//!
//! Discovery and resolution are two halves of one concept, so they live together. The
//! catalog owns the default language, which keeps callers from having to thread
//! configuration through every lookup.

mod discovery;
mod metadata;

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use crate::config::app_config::TesseractConfig;
use crate::models::{error::ErrorType, images::ImagesQueryParams, languages::LanguageSummary};

/// One `.traineddata` file on disk.
///
/// Internal to the catalog: [`LanguageSummary`] is the wire representation, so the
/// on-disk layout can change without altering the API contract.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TesseractModel {
    /// The Tesseract language code, such as `eng` or `chi_sim`.
    pub language: String,
    /// Distinguishes variants when a language has more than one model. `None` when the
    /// file sits directly in the tessdata directory.
    pub model: Option<String>,
    /// The file's path, including the configured tessdata directory.
    pub full_path: PathBuf,
    /// What Tesseract's `init` expects: the path relative to the tessdata directory,
    /// without the `.traineddata` extension.
    pub relative_path: String,
}

/// The set of Tesseract models discovered on disk.
#[derive(Debug)]
pub struct TessdataCatalog {
    models: HashSet<TesseractModel>,
    metadata: metadata::ModelMetadata,
    default_language: String,
}

impl TessdataCatalog {
    /// Discovers every Tesseract model under the configured tessdata directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the configured data path is not a directory.
    pub fn load(tesseract: &TesseractConfig, default_language: &str) -> io::Result<Self> {
        let root = Path::new(&tesseract.data_path);
        let models = discovery::discover(root)?;
        let metadata = metadata::ModelMetadata::load(root, &models)?;

        if models.is_empty() {
            tracing::warn!(
                "No Tesseract models found under '{}'. Every OCR request will be rejected \
                 until at least one .traineddata file is present.",
                tesseract.data_path
            );
        }

        Ok(Self {
            models,
            metadata,
            default_language: default_language.to_owned(),
        })
    }

    /// Resolves request parameters to exactly one model.
    ///
    /// Falls back to the configured default language when none is requested. For a
    /// language with several Models, it chooses the unnamed Model first and then an
    /// optional metadata default.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorType::InvalidRequest`] when a model is given without a language,
    /// the language is unavailable, the model is unknown for that language, or the
    /// language has several named models and none was chosen.
    pub fn resolve(&self, params: &ImagesQueryParams) -> Result<&TesseractModel, ErrorType> {
        // A model only means something in the context of a language.
        if params.model.is_some() && params.language.is_none() {
            return Err(ErrorType::InvalidRequest(
                "Language must be specified when model is provided".to_owned(),
            ));
        }

        let language = params.language.as_deref().unwrap_or(&self.default_language);

        let matching: Vec<&TesseractModel> = self
            .models
            .iter()
            .filter(|candidate| candidate.language == language)
            .collect();

        let Some(only_match) = matching.first().copied() else {
            return Err(ErrorType::InvalidRequest(format!(
                "Language '{language}' is not available"
            )));
        };

        if let Some(requested_model) = &params.model {
            return matching
                .iter()
                .find(|candidate| candidate.model.as_deref() == Some(requested_model.as_str()))
                .copied()
                .ok_or_else(|| {
                    ErrorType::InvalidRequest(format!(
                        "Model '{requested_model}' not found for language '{language}'"
                    ))
                });
        }

        if matching.len() == 1 {
            return Ok(only_match);
        }

        // Several models for this language: prefer the unnamed Model, then an optional
        // sidecar default, otherwise the caller has to choose.
        if let Some(unnamed_model) = matching
            .iter()
            .find(|candidate| candidate.model.is_none())
            .copied()
        {
            return Ok(unnamed_model);
        }

        if let Some(default_model) = self.metadata.default_for(language) {
            return matching
                .iter()
                .find(|candidate| candidate.model.as_deref() == Some(default_model))
                .copied()
                .ok_or_else(|| {
                    ErrorType::InvalidRequest(format!(
                        "Multiple models available for language '{language}', please specify a model"
                    ))
                });
        }

        Err(ErrorType::InvalidRequest(format!(
            "Multiple models available for language '{language}', please specify a model"
        )))
    }

    /// Every available model, sorted by language and then by model name.
    ///
    /// Ordering is the catalog's guarantee, not a caller's.
    #[must_use]
    pub fn list(&self) -> Vec<LanguageSummary> {
        let mut summaries: Vec<LanguageSummary> = self
            .models
            .iter()
            .map(|model| LanguageSummary {
                language: model.language.clone(),
                model: model.model.clone(),
                description: self
                    .metadata
                    .description_for(&model.language, model.model.as_deref())
                    .map(str::to_owned),
                is_default: self
                    .metadata
                    .is_default(&model.language, model.model.as_deref()),
            })
            .collect();
        summaries.sort();
        summaries
    }
}

#[cfg(test)]
mod tests {
    use super::{TessdataCatalog, TesseractModel};
    use crate::{
        config::app_config::TesseractConfig,
        models::{error::ErrorType, images::ImagesQueryParams},
    };
    use std::collections::HashSet;
    use std::fs::{self, File};
    use std::io::Write as _;
    use std::path::Path;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn model(language: &str, name: Option<&str>) -> TesseractModel {
        let relative_path = match name {
            Some(name) => format!("{language}/{name}"),
            None => language.to_owned(),
        };
        TesseractModel {
            language: language.to_owned(),
            model: name.map(str::to_owned),
            full_path: PathBuf::from(format!("tesseract/{relative_path}.traineddata")),
            relative_path,
        }
    }

    fn catalog(models: &[TesseractModel]) -> TessdataCatalog {
        TessdataCatalog {
            models: models.iter().cloned().collect::<HashSet<_>>(),
            metadata: super::metadata::ModelMetadata::default(),
            default_language: "eng".to_owned(),
        }
    }

    fn params(language: Option<&str>, name: Option<&str>) -> ImagesQueryParams {
        ImagesQueryParams {
            language: language.map(str::to_owned),
            model: name.map(str::to_owned),
            ..Default::default()
        }
    }

    fn create_file(directory: &Path, name: &str, contents: &[u8]) {
        let mut file = File::create(directory.join(name)).expect("could not create test file");
        file.write_all(contents).expect("could not write test file");
    }

    fn named_sme_models() -> TempDir {
        let tessdata = TempDir::new().expect("could not create tessdata directory");
        let language_directory = tessdata.path().join("sme");
        fs::create_dir(&language_directory).expect("could not create language directory");
        create_file(&language_directory, "sme_bible.traineddata", b"test data");
        create_file(&language_directory, "sme_dict.traineddata", b"test data");
        tessdata
    }

    fn load_catalog(tessdata: &TempDir) -> std::io::Result<TessdataCatalog> {
        TessdataCatalog::load(
            &TesseractConfig {
                data_path: tessdata.path().to_string_lossy().into_owned(),
            },
            "eng",
        )
    }

    /// Asserts that resolution failed with the given `InvalidRequest` message.
    #[track_caller]
    fn assert_invalid_request(result: Result<&TesseractModel, ErrorType>, expected: &str) {
        match result {
            Err(ErrorType::InvalidRequest(message)) => assert_eq!(message, expected),
            Err(other) => panic!("expected InvalidRequest, got {other:?}"),
            Ok(model) => panic!("expected InvalidRequest, got model {model:?}"),
        }
    }

    #[test]
    fn model_without_language_is_rejected() {
        let catalog = catalog(&[]);

        assert_invalid_request(
            catalog.resolve(&params(None, Some("fast"))),
            "Language must be specified when model is provided",
        );
    }

    #[test]
    fn unavailable_language_is_rejected() {
        let catalog = catalog(&[]);

        assert_invalid_request(
            catalog.resolve(&params(Some("xyz"), None)),
            "Language 'xyz' is not available",
        );
    }

    #[test]
    fn language_and_model_together_select_that_model() {
        let catalog = catalog(&[model("spa", Some("fast")), model("spa", Some("default"))]);

        let resolved = catalog
            .resolve(&params(Some("spa"), Some("fast")))
            .expect("expected a resolved model");

        assert_eq!(resolved.language, "spa");
        assert_eq!(resolved.model.as_deref(), Some("fast"));
    }

    #[test]
    fn unknown_model_for_a_known_language_is_rejected() {
        let catalog = catalog(&[model("spa", Some("fast"))]);

        assert_invalid_request(
            catalog.resolve(&params(Some("spa"), Some("slow"))),
            "Model 'slow' not found for language 'spa'",
        );
    }

    #[test]
    fn a_language_with_one_model_needs_no_model_name() {
        let catalog = catalog(&[model("spa", Some("fast"))]);

        let resolved = catalog
            .resolve(&params(Some("spa"), None))
            .expect("expected a resolved model");

        assert_eq!(resolved.language, "spa");
        assert_eq!(resolved.model.as_deref(), Some("fast"));
    }

    #[test]
    fn several_models_fall_back_to_the_unnamed_one() {
        let catalog = catalog(&[model("eng", None), model("eng", Some("fast"))]);

        let resolved = catalog
            .resolve(&params(Some("eng"), None))
            .expect("expected a resolved model");

        assert_eq!(resolved.language, "eng");
        assert_eq!(resolved.model, None);
    }

    #[test]
    fn several_named_models_require_an_explicit_choice() {
        let catalog = catalog(&[model("eng", Some("fast")), model("eng", Some("best"))]);

        assert_invalid_request(
            catalog.resolve(&params(Some("eng"), None)),
            "Multiple models available for language 'eng', please specify a model",
        );
    }

    #[test]
    fn no_language_falls_back_to_the_default_language() {
        let catalog = catalog(&[model("eng", None)]);

        let resolved = catalog
            .resolve(&params(None, None))
            .expect("expected a resolved model");

        assert_eq!(resolved.language, "eng");
        assert_eq!(resolved.model, None);
    }

    #[test]
    fn an_unavailable_default_language_is_reported_by_name() {
        let catalog = catalog(&[model("spa", None)]);

        assert_invalid_request(
            catalog.resolve(&params(None, None)),
            "Language 'eng' is not available",
        );
    }

    #[test]
    fn no_models_toml_keeps_named_models_requiring_an_explicit_choice() {
        let tessdata = named_sme_models();
        let catalog = load_catalog(&tessdata).expect("catalog should load without metadata");

        assert_invalid_request(
            catalog.resolve(&params(Some("sme"), None)),
            "Multiple models available for language 'sme', please specify a model",
        );

        let summaries = catalog.list();
        assert!(
            summaries
                .iter()
                .all(|summary| summary.description.is_none() && !summary.is_default)
        );
    }

    #[test]
    fn models_toml_default_resolves_a_language_without_a_model_parameter() {
        let tessdata = named_sme_models();
        create_file(
            &tessdata.path().join("sme"),
            "models.toml",
            br#"default = "sme_bible""#,
        );
        let catalog = load_catalog(&tessdata).expect("catalog should load with metadata");

        let resolved = catalog
            .resolve(&params(Some("sme"), None))
            .expect("metadata default should resolve");

        assert_eq!(resolved.model.as_deref(), Some("sme_bible"));
    }

    #[test]
    fn models_toml_default_for_a_missing_model_stops_startup() {
        let tessdata = named_sme_models();
        create_file(
            &tessdata.path().join("sme"),
            "models.toml",
            br#"default = "sme_missing""#,
        );

        let error = load_catalog(&tessdata).expect_err("missing metadata default must fail");
        let message = error.to_string();

        assert!(message.contains("models.toml"));
        assert!(message.contains("sme_missing"));
    }

    #[test]
    fn malformed_models_toml_stops_startup() {
        let tessdata = named_sme_models();
        create_file(&tessdata.path().join("sme"), "models.toml", b"default = [");

        let error = load_catalog(&tessdata).expect_err("malformed metadata must fail");
        let message = error.to_string();

        assert!(message.contains("models.toml"));
        assert!(message.contains("Could not parse model metadata file"));
    }

    #[test]
    fn models_toml_descriptions_surface_through_list() {
        let tessdata = named_sme_models();
        create_file(
            &tessdata.path().join("sme"),
            "models.toml",
            br#"default = "sme_bible"

[models.sme_bible]
description = "Trained on the 1895 North Sami bible"

[models.sme_dict]
description = "Trained on dictionary scans, 400dpi"
"#,
        );
        let catalog = load_catalog(&tessdata).expect("catalog should load with metadata");
        let summaries = catalog.list();

        let bible = summaries
            .iter()
            .find(|summary| summary.model.as_deref() == Some("sme_bible"))
            .expect("sme_bible should be listed");
        assert_eq!(
            bible.description.as_deref(),
            Some("Trained on the 1895 North Sami bible")
        );
        assert!(bible.is_default);

        let dictionary = summaries
            .iter()
            .find(|summary| summary.model.as_deref() == Some("sme_dict"))
            .expect("sme_dict should be listed");
        assert_eq!(
            dictionary.description.as_deref(),
            Some("Trained on dictionary scans, 400dpi")
        );
        assert!(!dictionary.is_default);
    }

    #[test]
    fn list_is_sorted_by_language_then_model() {
        let catalog = catalog(&[
            model("spa", Some("fast")),
            model("eng", Some("best")),
            model("eng", None),
            model("chi_sim", Some("vert")),
        ]);

        let listed: Vec<(String, Option<String>)> = catalog
            .list()
            .into_iter()
            .map(|summary| (summary.language, summary.model))
            .collect();

        assert_eq!(
            listed,
            vec![
                ("chi_sim".to_owned(), Some("vert".to_owned())),
                // The unnamed model sorts ahead of the named ones for the same language.
                ("eng".to_owned(), None),
                ("eng".to_owned(), Some("best".to_owned())),
                ("spa".to_owned(), Some("fast".to_owned())),
            ]
        );
    }
}
