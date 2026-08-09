#![expect(
    clippy::unwrap_used,
    reason = "test scaffolding: a failure here means the setup itself is broken, and panicking reports that directly"
)]

use std::fs::{self, File};
use std::io::Write as _;
use std::path::Path;

use axum::{Router, body::Body, http::Request, http::Response};
use http_body_util::BodyExt as _;
use tempfile::TempDir;
use tower::ServiceExt as _;

use ocr_service::{
    AppState,
    config::app_config::{AppConfig, TesseractConfig, app_config},
    router,
};

pub struct TestApp {
    pub router: Router,
}

impl TestApp {
    /// Builds a test app over the real environment configuration and the repository's
    /// `tesseract/` directory.
    #[expect(
        clippy::expect_used,
        reason = "these tests cannot run without the repository's tesseract/ directory"
    )]
    pub fn new() -> Self {
        // Loads the .env file located in the environment's current directory or its parents in sequence.
        // .env is used only for development, so a missing file is not an error.
        let _dotenv = dotenvy::dotenv();

        // Parse configuration from the environment.
        // This will exit with a help message if something is wrong.
        let app_config = app_config().to_owned();

        Self::with_state(
            AppState::from_config(app_config).expect("could not build the application state"),
        )
    }

    /// Builds a test app over a caller-supplied state, so tests can drive the HTTP
    /// surface against a synthetic Tessdata Catalog.
    pub fn with_state(state: AppState) -> Self {
        Self {
            router: router(state),
        }
    }

    pub async fn request(&self, req: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(req).await.unwrap()
    }
}

/// A test app over a synthetic Tessdata Catalog.
///
/// Lets tests reach the error branches of the images endpoint without depending on the
/// repository's real `tesseract/` directory. The model files hold placeholder bytes, so
/// only requests that fail before Tesseract is invoked are meaningful here.
pub struct SyntheticApp {
    pub app: TestApp,
    // Deleted on drop, so it has to outlive the app reading from it.
    _tessdata: TempDir,
}

impl SyntheticApp {
    /// A catalog holding `eng` plus `chi_sim` with two named models.
    pub fn new() -> Self {
        Self::with_config(|_app_config| {})
    }

    /// As [`SyntheticApp::new`], with a chance to adjust the configuration first.
    pub fn with_config(customise: impl FnOnce(&mut AppConfig)) -> Self {
        let tessdata = TempDir::new().unwrap();
        write_placeholder_model(tessdata.path(), "eng.traineddata");

        let chi_sim = tessdata.path().join("chi_sim");
        fs::create_dir(&chi_sim).unwrap();
        write_placeholder_model(&chi_sim, "chi_sim.traineddata");
        write_placeholder_model(&chi_sim, "chi_sim_vert.traineddata");

        let mut app_config = AppConfig {
            tesseract: TesseractConfig {
                data_path: tessdata.path().to_string_lossy().into_owned(),
            },
            ..AppConfig::default()
        };
        customise(&mut app_config);

        Self {
            app: TestApp::with_state(AppState::from_config(app_config).unwrap()),
            _tessdata: tessdata,
        }
    }
}

fn write_placeholder_model(dir: &Path, name: &str) {
    let mut file = File::create(dir.join(name)).unwrap();
    file.write_all(b"placeholder traineddata").unwrap();
}

/// The multipart boundary shared by the request helpers.
pub const BOUNDARY: &str = "test_boundary";

/// The `Content-Type` header value matching [`BOUNDARY`].
pub fn multipart_content_type() -> String {
    format!("multipart/form-data; boundary={BOUNDARY}")
}

/// Builds a multipart body with a single file field.
///
/// A `content_type` of `None` omits the part's own `Content-Type` header, which the
/// images endpoint has to reject.
pub fn multipart_body(filename: &str, content_type: Option<&str>, data: &[u8]) -> Body {
    let mut body = Vec::new();

    body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"image\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    if let Some(content_type) = content_type {
        body.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
    }
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());

    Body::from(body)
}

/// Reads the `message` field out of an error response body.
pub async fn error_message(response: Response<Body>) -> String {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    body.get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
