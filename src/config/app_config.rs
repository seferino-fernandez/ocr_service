use std::env;
use std::sync::OnceLock;
use std::time::Duration;

use super::error::ServerError;

const DEFAULT_SERVER_REQUEST_TIMEOUT: u64 = 15;
const DEFAULT_SERVER_HOST: &str = "0.0.0.0";
const DEFAULT_SERVER_PORT: u16 = 8080;
const DEFAULT_SERVER_FILE_UPLOAD_MAX_SIZE: usize = 1024 * 1024 * 10;
const DEFAULT_SERVER_ENVIRONMENT: &str = "development";

const DEFAULT_SERVICE_NAME: &str = "ocr-service";

const DEFAULT_MAX_ACCESS_CONTROL_AGE: u64 = 600;

const DEFAULT_TESSERACT_DATA_PATH: &str = "tesseract";

pub fn app_config() -> &'static AppConfig {
    static INSTANCE: OnceLock<AppConfig> = OnceLock::new();

    INSTANCE.get_or_init(|| {
        AppConfig::load_from_env()
            .unwrap_or_else(|ex| panic!("Unable to load application configuration: {ex:?}"))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub service: ServiceConfig,
    pub security: SecurityConfig,
    pub otel: OtelConfig,
    pub otel_provider: OtelProviderConfig,
    pub tesseract: TesseractConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub file_upload_max_size: usize,
    pub environment: String,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityConfig {
    pub max_access_control_age: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtelProviderConfig {
    pub provider: Option<String>,
    pub organization: Option<String>,
    pub stream_name: Option<String>,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtelConfig {
    pub enabled: bool,
    pub service_name: Option<String>,
    pub traces_endpoint: Option<String>,
    pub logs_endpoint: Option<String>,
    pub metrics_endpoint: Option<String>,
    pub metric_export_interval: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TesseractConfig {
    pub data_path: String,
}

impl AppConfig {
    fn load_from_env() -> Result<AppConfig, ServerError> {
        Ok(AppConfig {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or(DEFAULT_SERVER_HOST.to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or(DEFAULT_SERVER_PORT.to_string())
                    .parse::<u16>()
                    .unwrap_or(DEFAULT_SERVER_PORT),
                file_upload_max_size: env::var("SERVER_FILE_UPLOAD_MAX_SIZE")
                    .unwrap_or(DEFAULT_SERVER_FILE_UPLOAD_MAX_SIZE.to_string())
                    .parse::<usize>()
                    .unwrap_or(DEFAULT_SERVER_FILE_UPLOAD_MAX_SIZE),
                environment: env::var("SERVER_ENVIRONMENT")
                    .unwrap_or(DEFAULT_SERVER_ENVIRONMENT.to_string()),
                timeout: Duration::from_secs(
                    env::var("SERVER_REQUEST_TIMEOUT")
                        .unwrap_or(DEFAULT_SERVER_REQUEST_TIMEOUT.to_string())
                        .parse::<u64>()
                        .unwrap_or(DEFAULT_SERVER_REQUEST_TIMEOUT),
                ),
            },
            service: ServiceConfig {
                name: env::var("SERVICE_NAME").unwrap_or(DEFAULT_SERVICE_NAME.to_string()),
            },
            security: SecurityConfig {
                max_access_control_age: Duration::from_secs(
                    env::var("SECURITY_MAX_ACCESS_CONTROL_AGE")
                        .unwrap_or(DEFAULT_MAX_ACCESS_CONTROL_AGE.to_string())
                        .parse::<u64>()
                        .unwrap_or(DEFAULT_MAX_ACCESS_CONTROL_AGE),
                ),
            },
            otel: OtelConfig {
                enabled: env::var("OTEL_ENABLED")
                    .unwrap_or("false".to_string())
                    .parse::<bool>()
                    .unwrap_or(false),
                service_name: env::var("OTEL_SERVICE_NAME").ok(),
                traces_endpoint: env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").ok(),
                logs_endpoint: env::var("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT").ok(),
                metrics_endpoint: env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT").ok(),
                metric_export_interval: env::var("OTEL_METRIC_EXPORT_INTERVAL")
                    .ok()
                    .map(|interval| Duration::from_millis(interval.parse::<u64>().unwrap())),
            },
            otel_provider: OtelProviderConfig {
                provider: env::var("OTEL_PROVIDER").ok(),
                organization: env::var("OTEL_PROVIDER_ORGANIZATION").ok(),
                stream_name: env::var("OTEL_PROVIDER_STREAM_NAME").ok(),
                auth_token: env::var("OTEL_PROVIDER_AUTH_TOKEN").ok(),
            },
            tesseract: TesseractConfig {
                data_path: env::var("TESSDATA_PATH")
                    .unwrap_or(DEFAULT_TESSERACT_DATA_PATH.to_string()),
            },
        })
    }
}
