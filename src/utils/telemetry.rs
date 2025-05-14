use std::time::Duration;

use crate::config::app_config::AppConfig;
use anyhow::Error;
use opentelemetry::{self, KeyValue, global, trace::TracerProvider};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, SpanExporter, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{
    Resource,
    metrics::Temporality,
    propagation::TraceContextPropagator,
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::{
    resource::{DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_NAME},
    trace::{SERVER_ADDRESS, SERVER_PORT},
};
use tonic::metadata::{MetadataMap, MetadataValue};
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt::format::FmtSpan, layer::SubscriberExt as _,
};

const OTEL_PROVIDER_OPENOBSERVE: &str = "openobserve";

#[must_use = "Recommend holding with 'let _guard = ' pattern to ensure the final telemetry data is sent to the server"]
pub struct OtelGuard {
    tracer_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
    logging_provider: Option<opentelemetry_sdk::logs::SdkLoggerProvider>,
    meter_provider: Option<opentelemetry_sdk::metrics::SdkMeterProvider>,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Some(tracer_provider) = self.tracer_provider.take() {
            tracing::info!("Flushing OpenTelemetry traces");
            if let Err(err) = tracer_provider.force_flush() {
                eprintln!("Failed to flush OpenTelemetry traces: {err:?}");
            }
            tracing::info!("Shutting down OpenTelemetry tracer provider");
            if let Err(err) = tracer_provider.shutdown() {
                eprintln!("Failed to shutdown OpenTelemetry tracer provider: {err:?}");
            }
        }
        if let Some(logging_provider) = self.logging_provider.take() {
            tracing::info!("Flushing OpenTelemetry logs");
            if let Err(err) = logging_provider.force_flush() {
                eprintln!("Failed to flush OpenTelemetry logs: {err:?}");
            }
            tracing::info!("Shutting down OpenTelemetry logging provider");
            if let Err(err) = logging_provider.shutdown() {
                eprintln!("Failed to shutdown OpenTelemetry logging provider: {err:?}");
            }
        }
        if let Some(meter_provider) = self.meter_provider.take() {
            tracing::info!("Flushing OpenTelemetry metrics");
            if let Err(err) = meter_provider.force_flush() {
                eprintln!("Failed to flush OpenTelemetry metrics: {err:?}");
            }
            tracing::info!("Shutting down OpenTelemetry meter provider");
            if let Err(err) = meter_provider.shutdown() {
                eprintln!("Failed to shutdown OpenTelemetry meter provider: {err:?}");
            }
        }
    }
}

pub async fn initialize_opentelemetry_providers(
    app_config: &AppConfig,
) -> Result<OtelGuard, Error> {
    if !app_config.otel.enabled {
        tracing::info!("OpenTelemetry is disabled, only stdout logging will be used");
        let stdout_fmt_layer = stdout_layer(app_config);
        let subscriber = Registry::default().with(stdout_fmt_layer);
        tracing::subscriber::set_global_default(subscriber)
            .expect("Could not set up global logger");

        return Ok(OtelGuard {
            tracer_provider: None,
            logging_provider: None,
            meter_provider: None,
        });
    }
    tracing::info!(
        "Initializing OpenTelemetry with traces endpoint: {}",
        app_config.otel.traces_endpoint.as_ref().unwrap()
    );

    global::set_text_map_propagator(TraceContextPropagator::new());

    // Initialize OpenTelemetry Logging provider
    let logging_provider = init_logging_provider(app_config)?;
    let otel_logging_layer =
        OpenTelemetryTracingBridge::new(&logging_provider).with_filter(otel_env_filter());

    // Initialize OpenTelemetry Tracing provider
    let tracer_provider = init_tracer_provider(app_config)?;
    let tracer = tracer_provider.tracer(app_config.service.name.clone());
    let otel_tracing_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_error_records_to_exceptions(true)
        .with_filter(otel_env_filter());

    let stdout_fmt_layer = stdout_layer(app_config);

    // Initialize OpenTelemetry Metrics provider
    let meter_provider = init_meter_provider(app_config)?;

    let subscriber = Registry::default()
        .with(stdout_fmt_layer)
        .with(otel_logging_layer)
        .with(otel_tracing_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set up global logger");
    tracing::info!("OpenTelemetry initialization complete");
    Ok(OtelGuard {
        tracer_provider: Some(tracer_provider),
        logging_provider: Some(logging_provider),
        meter_provider: Some(meter_provider),
    })
}

// Copied from https://github.com/open-telemetry/opentelemetry-rust/blob/main/opentelemetry-otlp/examples/basic-otlp/src/main.rs
//
// For the OpenTelemetry layer, add a tracing filter to filter events from
// OpenTelemetry and its dependent crates (opentelemetry-otlp uses crates
// like reqwest/tonic etc.) from being sent back to OTel itself, thus
// preventing infinite telemetry generation. The filter levels are set as
// follows:
// - Allow `info` level and above by default.
// - Restrict `opentelemetry`, `hyper`, `tonic`, and `reqwest` completely.
// Note: This will also drop events from crates like `tonic` etc. even when
// they are used outside the OTLP Exporter. For more details, see:
// https://github.com/open-telemetry/opentelemetry-rust/issues/761
fn otel_env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info".into())
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("opentelemetry=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap())
}

fn stdout_layer(app_config: &AppConfig) -> impl Layer<tracing_subscriber::Registry> {
    tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_thread_ids(true)
        .with_level(true)
        .with_file(true)
        .with_line_number(true)
        // Only log spans when otel is enabled
        .with_span_events(if app_config.otel.enabled {
            FmtSpan::NEW | FmtSpan::CLOSE
        } else {
            FmtSpan::NONE
        })
        .with_filter(stdout_env_filter())
}

// Create a new tracing::Fmt layer to print the logs to stdout. It has a
// default filter of `info` level and above, and `debug` and above for logs
// from OpenTelemetry crates. The filter levels can be customized as needed.
fn stdout_env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info".into())
        .add_directive("opentelemetry=debug".parse().unwrap())
}

fn init_otel_resources(app_config: &AppConfig) -> Resource {
    Resource::builder()
        .with_service_name(app_config.service.name.clone())
        .with_attributes(vec![
            KeyValue::new(SERVICE_NAME, app_config.service.name.clone()),
            KeyValue::new(
                DEPLOYMENT_ENVIRONMENT_NAME,
                app_config.server.environment.clone(),
            ),
            KeyValue::new(SERVER_ADDRESS, app_config.server.host.clone()),
            KeyValue::new(SERVER_PORT, app_config.server.port.to_string()),
        ])
        .build()
}

fn init_tracer_provider(
    app_config: &AppConfig,
) -> Result<opentelemetry_sdk::trace::SdkTracerProvider, Error> {
    let span_exporter = init_span_exporter(app_config)?;
    let tracer_provider = SdkTracerProvider::builder()
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            1.0,
        ))))
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(init_otel_resources(app_config))
        .with_batch_exporter(span_exporter)
        .build();
    global::set_tracer_provider(tracer_provider.clone());
    Ok(tracer_provider)
}

fn init_span_exporter(app_config: &AppConfig) -> Result<SpanExporter, Error> {
    let mut builder = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(app_config.otel.traces_endpoint.clone().unwrap());
    if let Some(metadata) = get_metadata_map(app_config) {
        builder = builder.with_metadata(metadata);
    }
    let span_exporter = builder.build()?;
    Ok(span_exporter)
}

fn init_logging_provider(
    app_config: &AppConfig,
) -> Result<opentelemetry_sdk::logs::SdkLoggerProvider, Error> {
    let mut builder = LogExporter::builder()
        .with_tonic()
        .with_endpoint(app_config.otel.logs_endpoint.clone().unwrap());

    if let Some(metadata) = get_metadata_map(app_config) {
        builder = builder.with_metadata(metadata);
    }
    let logs_exporter = builder.build()?;

    let logger_provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder()
        .with_resource(init_otel_resources(app_config))
        .with_batch_exporter(logs_exporter)
        .build();
    Ok(logger_provider)
}

fn get_metadata_map(app_config: &AppConfig) -> Option<MetadataMap> {
    if app_config.otel_provider.provider == Some(OTEL_PROVIDER_OPENOBSERVE.to_string()) {
        return Some(openobserve_metadata(app_config));
    }
    None
}

fn openobserve_metadata(app_config: &AppConfig) -> MetadataMap {
    let mut map = MetadataMap::with_capacity(3);
    if let Some(auth_token) = &app_config.otel_provider.auth_token {
        map.insert(
            "authorization",
            MetadataValue::try_from(auth_token).unwrap(),
        );
    }
    if let Some(organization) = &app_config.otel_provider.organization {
        map.insert(
            "organization",
            MetadataValue::try_from(organization).unwrap(),
        );
    }
    if let Some(stream_name) = &app_config.otel_provider.stream_name {
        map.insert("stream-name", MetadataValue::try_from(stream_name).unwrap());
    }
    map
}

fn init_meter_provider(
    app_config: &AppConfig,
) -> Result<opentelemetry_sdk::metrics::SdkMeterProvider, Error> {
    let mut builder = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_temporality(Temporality::Cumulative)
        .with_endpoint(app_config.otel.metrics_endpoint.clone().unwrap())
        .with_timeout(Duration::from_secs(3));

    if let Some(metadata) = get_metadata_map(app_config) {
        builder = builder.with_metadata(metadata);
    }
    let metric_exporter = builder.build()?;

    let periodic_reader = opentelemetry_sdk::metrics::PeriodicReader::builder(metric_exporter)
        .with_interval(app_config.otel.metric_export_interval.unwrap())
        .build();

    let meter_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_resource(init_otel_resources(app_config))
        .with_reader(periodic_reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    Ok(meter_provider)
}
