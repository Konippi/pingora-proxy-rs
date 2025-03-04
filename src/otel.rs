use std::sync::Arc;

use async_trait::async_trait;
use opentelemetry::{
    KeyValue,
    global::{self},
    trace::TracerProvider,
};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{
    LogExporter, MetricExporter, SpanExporter, WithExportConfig,
};
use opentelemetry_sdk::{
    Resource,
    logs::SdkLoggerProvider,
    metrics::{SdkMeterProvider, Temporality},
    trace::{self, RandomIdGenerator, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::{
    SCHEMA_URL,
    resource::{SERVICE_NAME, SERVICE_VERSION},
};
use pingora::{server::ShutdownWatch, services::background::BackgroundService};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    fmt::{self},
    prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt,
};

use crate::config::CONFIG;

pub struct OtelGuard {
    logger_provider: SdkLoggerProvider,
    tracer_provider: SdkTracerProvider,
    metrics_provider: SdkMeterProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        self.tracer_provider
            .shutdown()
            .expect("Failed to shutdown tracer provider");
        self.metrics_provider
            .shutdown()
            .expect("Failed to shutdown metrics provider");
        self.logger_provider
            .shutdown()
            .expect("Failed to shutdown logger provider");
    }
}

pub struct OtelService;

#[async_trait]
impl BackgroundService for OtelService {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        match self.start_instrument() {
            Ok(otel_guard) => {
                let _otel_guard = Arc::new(otel_guard);
                tracing::info!("OpenTelemetry instrumentation started.");

                if let Err(e) = shutdown.changed().await {
                    tracing::error!("Error during shutdown: {}", e);
                }
                tracing::info!("OpenTelemetry instrumentation shutting down.");
            }
            Err(e) => {
                tracing::error!(
                    "Failed to start OpenTelemetry instrumentation: {:#}",
                    e
                );
            }
        }
    }
}

impl OtelService {
    pub fn start_instrument(&self) -> anyhow::Result<OtelGuard> {
        let resource = build_resource();

        let fmt_layer = build_fmt_layer();

        let logger_provider = build_logger_provider(&resource)?;
        let logger_layer = OpenTelemetryTracingBridge::new(&logger_provider)
            .with_filter(
                EnvFilter::try_from_default_env()
                    .or_else(|_| EnvFilter::try_new("info"))?,
            );

        let tracer_provider = build_tracer_provider(&resource)?;
        let tracer =
            tracer_provider.tracer(format!("{}-tracer", CONFIG.package_name));
        let tracer_layer = OpenTelemetryLayer::new(tracer);

        let metrics_provider = build_metrics_provider(&resource)?;
        let metrics_layer = MetricsLayer::new(metrics_provider.clone());

        Registry::default()
            .with(fmt_layer)
            .with(logger_layer)
            .with(tracer_layer)
            .with(metrics_layer)
            .try_init()?;

        Ok(OtelGuard {
            logger_provider,
            tracer_provider,
            metrics_provider,
        })
    }
}

/// Build a resource.
fn build_resource() -> Resource {
    Resource::builder()
        .with_schema_url(
            [
                KeyValue::new(SERVICE_NAME, CONFIG.package_name.as_str()),
                KeyValue::new(SERVICE_VERSION, CONFIG.package_version.as_str()),
            ],
            SCHEMA_URL,
        )
        .build()
}

/// Build a fmt layer for tracing subscriber.
fn build_fmt_layer() -> fmt::Layer<Registry> {
    fmt::Layer::new()
        .with_ansi(CONFIG.tracing_subscriber_fmt_color)
        .with_file(CONFIG.tracing_subscriber_fmt_file)
        .with_line_number(CONFIG.tracing_subscriber_fmt_line_number)
        .with_target(CONFIG.tracing_subscriber_fmt_target)
        .with_thread_names(CONFIG.tracing_subscriber_fmt_thread_names)
}

/// Build a logger provider.
fn build_logger_provider(
    resource: &Resource,
) -> anyhow::Result<SdkLoggerProvider> {
    let exporter = LogExporter::builder()
        .with_tonic()
        .with_endpoint(CONFIG.otel_log_exporter_endpoint.as_str())
        .build()?;
    let provider = SdkLoggerProvider::builder()
        .with_resource(resource.clone())
        .with_simple_exporter(exporter)
        .build();

    Ok(provider)
}

/// Build a tracer provider.
fn build_tracer_provider(
    resource: &Resource,
) -> anyhow::Result<SdkTracerProvider> {
    let id_generator = RandomIdGenerator::default();
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(CONFIG.otel_trace_exporter_endpoint.as_str())
        .build()?;
    let processor_config = trace::BatchConfigBuilder::default()
        .with_max_queue_size(CONFIG.otel_trace_processor_max_queue_size)
        .with_scheduled_delay(CONFIG.otel_trace_processor_scheduled_delay)
        .with_max_export_batch_size(
            CONFIG.otel_trace_processor_max_export_batch_size,
        )
        .with_max_export_timeout(CONFIG.otel_trace_processor_max_export_timeout)
        .with_max_concurrent_exports(
            CONFIG.otel_trace_processor_max_concurrent_exports,
        )
        .build();
    let batch_span_processor = trace::BatchSpanProcessor::builder(exporter)
        .with_batch_config(processor_config)
        .build();
    let provider = SdkTracerProvider::builder()
        .with_id_generator(id_generator)
        .with_resource(resource.clone())
        .with_span_processor(batch_span_processor)
        .build();

    global::set_tracer_provider(provider.clone());

    Ok(provider)
}

/// Build a metrics provider.
fn build_metrics_provider(
    resource: &Resource,
) -> anyhow::Result<SdkMeterProvider> {
    let exporter = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(CONFIG.otel_metrics_exporter_endpoint.as_str())
        .with_temporality(Temporality::Cumulative)
        .build()?;
    let provider = SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_periodic_exporter(exporter)
        .build();

    global::set_meter_provider(provider.clone());

    Ok(provider)
}
