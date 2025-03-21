use crate::config::app_config::AppConfig;
use crate::models::languages::TesseractModel;
use std::collections::HashSet;
use std::io;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

/// Get all available languages with their models from the tesseract data path.
pub fn get_available_languages_with_models(
    app_config: &AppConfig,
) -> io::Result<HashSet<TesseractModel>> {
    let tesseract_data_path = Path::new(&app_config.tesseract.data_path);
    let mut languages: HashSet<TesseractModel> = HashSet::new();
    let mut languages_visited: HashSet<String> = HashSet::new();

    for entry_result in WalkDir::new(tesseract_data_path)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .filter(is_non_hidden_file)
        .filter(is_traineddata_file)
    {
        let file_name = entry_result
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let language_model_name = file_name.trim_end_matches(".traineddata");
        // The full path is the tesseract file path including the $TESSDATA_PREFIX
        let full_path = entry_result.path().to_string_lossy().to_string();
        // The relative path is the file name without the .traineddata extension or the $TESSDATA_PREFIX
        let relative_path = full_path
            .strip_prefix(&app_config.tesseract.data_path)
            // Also remove the leading '/' if there is one
            .map(|s| s.strip_prefix("/").unwrap_or(s))
            .unwrap_or_default()
            .strip_suffix(".traineddata")
            .unwrap_or_default()
            .to_string();
        match entry_result.depth() {
            1 => {
                // Files directly in tesseract data path
                if languages_visited.insert(language_model_name.to_string()) {
                    languages.insert(TesseractModel {
                        language: language_model_name.to_string(),
                        model: None,
                        full_path: Some(full_path),
                        relative_path: Some(relative_path),
                    });
                }
            }
            2 => {
                // Files in subdirectories (language directories)
                if let Some(language_dir_name) = entry_result
                    .path()
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|name| name.to_str())
                {
                    if languages_visited
                        .insert(format!("{}_{}", language_dir_name, language_model_name))
                    {
                        languages.insert(TesseractModel {
                            language: language_dir_name.to_string(),
                            model: Some(language_model_name.to_string()),
                            full_path: Some(full_path),
                            relative_path: Some(relative_path),
                        });
                    }
                }
            }
            _ => {
                // Ignore other depths
            }
        }
    }
    Ok(languages)
}

fn is_non_hidden_file(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| !s.starts_with("."))
        .unwrap_or(false)
}

fn is_traineddata_file(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.ends_with(".traineddata"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::{self, Write};
    use std::path::Path;
    use std::time::Duration;
    use tempfile;

    use crate::config::app_config::AppConfig;
    use crate::utils::languages::get_available_languages_with_models;

    fn create_test_file(dir: &Path, name: &str) -> io::Result<()> {
        let path = dir.join(name);
        let mut file = File::create(path)?;
        file.write_all(b"test data")?;
        Ok(())
    }

    fn create_test_config(data_path: String) -> AppConfig {
        AppConfig {
            server: crate::config::app_config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                file_upload_max_size: 1024 * 1024 * 10,
                file_upload_max_size_enabled: true,
                environment: "test".to_string(),
                timeout: Duration::from_secs(15),
            },
            service: crate::config::app_config::ServiceConfig {
                name: "test-service".to_string(),
                default_language: "eng".to_string(),
            },
            security: crate::config::app_config::SecurityConfig {
                max_access_control_age: Duration::from_secs(600),
            },
            otel: crate::config::app_config::OtelConfig {
                enabled: false,
                service_name: None,
                traces_endpoint: None,
                logs_endpoint: None,
                metrics_endpoint: None,
                metric_export_interval: None,
            },
            otel_provider: crate::config::app_config::OtelProviderConfig {
                provider: None,
                organization: None,
                stream_name: None,
                auth_token: None,
            },
            tesseract: crate::config::app_config::TesseractConfig { data_path },
        }
    }

    #[test]
    fn test_hashset_collection() -> io::Result<()> {
        // Create a temporary directory structure
        let temp_dir = tempfile::TempDir::new()?;

        // Create root language files
        create_test_file(temp_dir.path(), "eng.traineddata")?;
        create_test_file(temp_dir.path(), "fra.traineddata")?;

        // Create a subdirectory with model files
        let model_dir = temp_dir.path().join("chi_sim");
        fs::create_dir(&model_dir)?;
        create_test_file(&model_dir, "fast.traineddata")?;
        create_test_file(&model_dir, "best.traineddata")?;

        // Create some invalid files to test filtering
        create_test_file(temp_dir.path(), "invalid.txt")?;
        create_test_file(temp_dir.path(), ".hidden.traineddata")?;

        let app_config = create_test_config(temp_dir.path().to_string_lossy().to_string());
        let actual_models = get_available_languages_with_models(&app_config)?;

        println!("HashSet created successfully: {:?}", actual_models);
        assert_eq!(actual_models.len(), 4);
        Ok(())
    }
}
