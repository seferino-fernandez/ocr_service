use std::collections::HashSet;
use std::io;
use std::path::Path;

use walkdir::{DirEntry, WalkDir};

use super::TesseractModel;

const TRAINEDDATA_EXTENSION: &str = ".traineddata";

/// Walks the tessdata directory and collects every Tesseract model it holds.
///
/// Files directly in the directory are models with no model name — `eng.traineddata`
/// becomes language `eng`. Files one level down are named variants of the directory's
/// language — `chi_sim/chi_sim_vert.traineddata` becomes language `chi_sim`, model
/// `chi_sim_vert`.
///
/// # Errors
///
/// Returns an error if `root` is not a directory. Individual unreadable entries are
/// skipped rather than failing the whole scan.
pub(super) fn discover(root: &Path) -> io::Result<HashSet<TesseractModel>> {
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Tesseract data path '{}' is not a directory. Set TESSDATA_PATH to the \
                 directory holding your .traineddata files.",
                root.display()
            ),
        ));
    }

    let models = WalkDir::new(root)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .filter(is_non_hidden_file)
        .filter(is_traineddata_file)
        .filter_map(|entry| model_from_entry(root, &entry))
        .collect();

    Ok(models)
}

/// Builds a model from a discovered file, or `None` if the path cannot be interpreted.
fn model_from_entry(root: &Path, entry: &DirEntry) -> Option<TesseractModel> {
    let path = entry.path();
    let file_stem = path
        .file_name()?
        .to_str()?
        .strip_suffix(TRAINEDDATA_EXTENSION)?;

    // What Tesseract's `init` expects: the path below the tessdata directory with the
    // extension removed. Derived from the real path rather than by trimming strings, so
    // a mismatch skips the entry instead of silently yielding an empty language.
    let relative_path = path
        .strip_prefix(root)
        .ok()?
        .with_extension("")
        .to_str()?
        .to_owned();

    match entry.depth() {
        1 => Some(TesseractModel {
            language: file_stem.to_owned(),
            model: None,
            full_path: path.to_path_buf(),
            relative_path,
        }),
        2 => Some(TesseractModel {
            language: path.parent()?.file_name()?.to_str()?.to_owned(),
            model: Some(file_stem.to_owned()),
            full_path: path.to_path_buf(),
            relative_path,
        }),
        _ => None,
    }
}

fn is_non_hidden_file(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| !name.starts_with('.'))
}

fn is_traineddata_file(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| name.ends_with(TRAINEDDATA_EXTENSION))
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::Write as _;
    use std::path::{Path, PathBuf};

    use super::discover;

    fn create_test_file(dir: &Path, name: &str) {
        let mut file = File::create(dir.join(name)).expect("could not create test file");
        file.write_all(b"test data")
            .expect("could not write test file");
    }

    /// A tessdata directory holding two root-level languages and one language with two
    /// named models, plus files that must be filtered out.
    fn fixture() -> tempfile::TempDir {
        let temp_dir = tempfile::TempDir::new().expect("could not create temp dir");
        let root = temp_dir.path();

        create_test_file(root, "eng.traineddata");
        create_test_file(root, "fra.traineddata");

        let model_dir = root.join("chi_sim");
        fs::create_dir(&model_dir).expect("could not create model dir");
        create_test_file(&model_dir, "fast.traineddata");
        create_test_file(&model_dir, "best.traineddata");

        // Neither of these is a model.
        create_test_file(root, "invalid.txt");
        create_test_file(root, ".hidden.traineddata");

        temp_dir
    }

    #[test]
    fn discovers_root_languages_and_subdirectory_models() {
        let temp_dir = fixture();
        let models = discover(temp_dir.path()).expect("discovery failed");

        assert_eq!(models.len(), 4);

        let mut found: Vec<(String, Option<String>)> = models
            .iter()
            .map(|model| (model.language.clone(), model.model.clone()))
            .collect();
        found.sort();

        assert_eq!(
            found,
            vec![
                ("chi_sim".to_owned(), Some("best".to_owned())),
                ("chi_sim".to_owned(), Some("fast".to_owned())),
                ("eng".to_owned(), None),
                ("fra".to_owned(), None),
            ]
        );
    }

    #[test]
    fn relative_path_is_what_tesseract_init_expects() {
        let temp_dir = fixture();
        let models = discover(temp_dir.path()).expect("discovery failed");

        let root_model = models
            .iter()
            .find(|model| model.language == "eng")
            .expect("eng model missing");
        assert_eq!(root_model.relative_path, "eng");
        assert_eq!(
            root_model.full_path,
            temp_dir.path().join("eng.traineddata")
        );

        let nested_model = models
            .iter()
            .find(|model| model.model.as_deref() == Some("fast"))
            .expect("chi_sim/fast model missing");
        assert_eq!(
            nested_model.relative_path,
            PathBuf::from("chi_sim").join("fast").to_string_lossy()
        );
    }

    #[test]
    fn missing_directory_is_an_error_rather_than_an_empty_catalog() {
        let temp_dir = tempfile::TempDir::new().expect("could not create temp dir");
        let missing = temp_dir.path().join("does-not-exist");

        let error = discover(&missing).expect_err("expected a NotFound error");
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn models_deeper_than_one_subdirectory_are_ignored() {
        let temp_dir = tempfile::TempDir::new().expect("could not create temp dir");
        let deep = temp_dir.path().join("a").join("b");
        fs::create_dir_all(&deep).expect("could not create nested dirs");
        create_test_file(&deep, "eng.traineddata");

        let models = discover(temp_dir.path()).expect("discovery failed");
        assert!(models.is_empty());
    }
}
