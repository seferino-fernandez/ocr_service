use axum::{Router, body::Body, http::Request, http::Response};
use tower::ServiceExt as _;

use ocr_service::{config::app_config::app_config, router};

pub struct TestApp {
    pub router: Router,
}

impl TestApp {
    pub fn new() -> Self {
        // Loads the .env file located in the environment's current directory or its parents in sequence.
        // .env used only for development, so we discard error in all other cases.
        dotenvy::dotenv().ok();

        // Set port to 0 so tests can spawn multiple servers on OS assigned ports.
        std::env::set_var("PORT", "0");

        // Parse configuration from the environment.
        // This will exit with a help message if something is wrong.
        let app_config = app_config().to_owned();

        let router = router(app_config);
        Self { router }
    }

    pub async fn request(&self, req: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(req).await.unwrap()
    }
}
