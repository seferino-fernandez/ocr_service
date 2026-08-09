//! Optional, per-language metadata for Tesseract Models.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

use super::TesseractModel;

const SIDECAR_FILE_NAME: &str = "models.toml";

/// Metadata loaded from every `models.toml` found in a language directory.
#[derive(Debug, Default)]
pub(super) struct ModelMetadata {
    default_models: HashMap<String, String>,
    descriptions: HashMap<String, HashMap<String, String>>,
}

impl ModelMetadata {
    /// Reads each language directory's optional metadata sidecar and validates defaults.
    pub(super) fn load(root: &Path, models: &HashSet<TesseractModel>) -> io::Result<Self> {
        let mut metadata = Self::default();

        for entry in fs::read_dir(root)?.filter_map(Result::ok) {
            let language_directory = entry.path();
            if !language_directory.is_dir() {
                continue;
            }

            let sidecar_path = language_directory.join(SIDECAR_FILE_NAME);
            if !sidecar_path.is_file() {
                continue;
            }

            let language = entry.file_name().into_string().map_err(|name| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Model metadata file '{}': language directory name is not valid UTF-8: {name:?}",
                        sidecar_path.display()
                    ),
                )
            })?;
            let contents = fs::read_to_string(&sidecar_path)?;
            let sidecar: LanguageMetadataFile = toml::from_str(&contents).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Could not parse model metadata file '{}': {error}",
                        sidecar_path.display()
                    ),
                )
            })?;

            if let Some(default_model) = sidecar.default {
                let default_exists = models.iter().any(|candidate| {
                    candidate.language == language
                        && candidate.model.as_deref() == Some(default_model.as_str())
                });
                if !default_exists {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Model metadata file '{}': default model '{default_model}' does not exist for language '{language}'",
                            sidecar_path.display()
                        ),
                    ));
                }
                metadata
                    .default_models
                    .insert(language.clone(), default_model);
            }

            for (model, model_metadata) in sidecar.models {
                if let Some(description) = model_metadata.description {
                    metadata
                        .descriptions
                        .entry(language.clone())
                        .or_default()
                        .insert(model, description);
                }
            }
        }

        Ok(metadata)
    }

    pub(super) fn default_for(&self, language: &str) -> Option<&str> {
        self.default_models.get(language).map(String::as_str)
    }

    pub(super) fn description_for(&self, language: &str, model: Option<&str>) -> Option<&str> {
        model.and_then(|model| {
            self.descriptions
                .get(language)
                .and_then(|descriptions| descriptions.get(model))
                .map(String::as_str)
        })
    }

    pub(super) fn is_default(&self, language: &str, model: Option<&str>) -> bool {
        model.is_some_and(|model| self.default_for(language) == Some(model))
    }
}

/// The TOML schema accepted in an individual language directory.
#[derive(Deserialize)]
struct LanguageMetadataFile {
    default: Option<String>,
    #[serde(default)]
    models: HashMap<String, ModelMetadataFile>,
}

#[derive(Deserialize)]
struct ModelMetadataFile {
    description: Option<String>,
}
