use std::env;
use std::sync::OnceLock;
use std::time::Duration;

const DEFAULT_SERVER_REQUEST_TIMEOUT: u64 = 15;
const DEFAULT_SERVER_HOST: &str = "0.0.0.0";
const DEFAULT_SERVER_PORT: u16 = 8080;
const DEFAULT_SERVER_FILE_UPLOAD_MAX_SIZE: usize = 1024 * 1024 * 10;
const DEFAULT_SERVER_FILE_UPLOAD_MAX_SIZE_ENABLED: bool = true;
const DEFAULT_SERVER_ENVIRONMENT: &str = "development";

const DEFAULT_SERVICE_NAME: &str = "ocr-service";
const DEFAULT_SERVICE_DEFAULT_LANGUAGE: &str = "eng";

const DEFAULT_MAX_ACCESS_CONTROL_AGE: u64 = 600; // 10 minutes

const DEFAULT_TESSERACT_DATA_PATH: &str = "tesseract";

/// Interval between OpenTelemetry metric exports when `OTEL_METRIC_EXPORT_INTERVAL` is unset.
pub const DEFAULT_OTEL_METRIC_EXPORT_INTERVAL: Duration = Duration::from_millis(10_000);

/// The process-wide application configuration, read from the environment on first use.
pub fn app_config() -> &'static AppConfig {
    static INSTANCE: OnceLock<AppConfig> = OnceLock::new();

    INSTANCE.get_or_init(AppConfig::load_from_env)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    pub file_upload_max_size_enabled: bool,
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
    pub default_language: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OtelProviderConfig {
    pub provider: Option<String>,
    pub organization: Option<String>,
    pub stream_name: Option<String>,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtelConfig {
    pub sdk_disabled: bool,
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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_SERVER_HOST.to_owned(),
            port: DEFAULT_SERVER_PORT,
            file_upload_max_size: DEFAULT_SERVER_FILE_UPLOAD_MAX_SIZE,
            file_upload_max_size_enabled: DEFAULT_SERVER_FILE_UPLOAD_MAX_SIZE_ENABLED,
            environment: DEFAULT_SERVER_ENVIRONMENT.to_owned(),
            timeout: Duration::from_secs(DEFAULT_SERVER_REQUEST_TIMEOUT),
        }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: DEFAULT_SERVICE_NAME.to_owned(),
            default_language: DEFAULT_SERVICE_DEFAULT_LANGUAGE.to_owned(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_access_control_age: Duration::from_secs(DEFAULT_MAX_ACCESS_CONTROL_AGE),
        }
    }
}

impl Default for OtelConfig {
    /// Defaults to a disabled SDK, so constructing a config without touching the
    /// environment never attempts to export telemetry.
    fn default() -> Self {
        Self {
            sdk_disabled: true,
            service_name: None,
            traces_endpoint: None,
            logs_endpoint: None,
            metrics_endpoint: None,
            metric_export_interval: None,
        }
    }
}

impl Default for TesseractConfig {
    fn default() -> Self {
        Self {
            data_path: DEFAULT_TESSERACT_DATA_PATH.to_owned(),
        }
    }
}

impl AppConfig {
    /// Reads the application configuration from the environment.
    ///
    /// Every field falls back to its documented default, so this cannot fail.
    #[must_use]
    pub fn load_from_env() -> AppConfig {
        AppConfig {
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
                file_upload_max_size_enabled: env::var("SERVER_FILE_UPLOAD_MAX_SIZE_ENABLED")
                    .unwrap_or(DEFAULT_SERVER_FILE_UPLOAD_MAX_SIZE_ENABLED.to_string())
                    .parse::<bool>()
                    .unwrap_or(DEFAULT_SERVER_FILE_UPLOAD_MAX_SIZE_ENABLED),
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
                default_language: env::var("SERVICE_DEFAULT_LANGUAGE")
                    .unwrap_or(DEFAULT_SERVICE_DEFAULT_LANGUAGE.to_string()),
            },
            security: SecurityConfig {
                max_access_control_age: Duration::from_secs(
                    env::var("SECURITY_MAX_ACCESS_CONTROL_AGE")
                        .unwrap_or(DEFAULT_MAX_ACCESS_CONTROL_AGE.to_string())
                        .parse::<u64>()
                        .unwrap_or(DEFAULT_MAX_ACCESS_CONTROL_AGE),
                ),
            },
            otel: Self::load_otel_config(),
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
        }
    }

    /// Loads OpenTelemetry related configuration from environment variables.
    fn load_otel_config() -> OtelConfig {
        // Determine if the OTel SDK is explicitly disabled by the environment variable.
        // OTEL_SDK_DISABLED=true means the SDK is disabled.
        // If OTEL_SDK_DISABLED is not set or is "false", the SDK is considered enabled by default.
        let is_otel_sdk_explicitly_disabled: bool = env::var("OTEL_SDK_DISABLED")
            .ok()
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);

        if is_otel_sdk_explicitly_disabled {
            // SDK is disabled, populate OtelConfig with default "disabled" values.
            OtelConfig {
                sdk_disabled: true,
                service_name: None,
                traces_endpoint: None,
                logs_endpoint: None,
                metrics_endpoint: None,
                metric_export_interval: None,
            }
        } else {
            // SDK is enabled (or not explicitly disabled), parse related OTel environment variables.
            let common_otlp_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();

            OtelConfig {
                sdk_disabled: false,
                service_name: env::var("OTEL_SERVICE_NAME").ok(),
                traces_endpoint: env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
                    .ok()
                    .or_else(|| common_otlp_endpoint.clone()),
                logs_endpoint: env::var("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT")
                    .ok()
                    .or_else(|| common_otlp_endpoint.clone()),
                metrics_endpoint: env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")
                    .ok()
                    .or(common_otlp_endpoint),
                metric_export_interval: Some(
                    env::var("OTEL_METRIC_EXPORT_INTERVAL")
                        .ok()
                        .and_then(|interval_str| interval_str.parse::<u64>().ok())
                        .map_or(DEFAULT_OTEL_METRIC_EXPORT_INTERVAL, Duration::from_millis),
                ),
            }
        }
    }
}
