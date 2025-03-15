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
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    fmt::{self},
    prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt,
};

use crate::config::CONFIG;

#[derive(Debug)]
pub struct OtelGuard {
    logger_provider: SdkLoggerProvider,
    tracer_provider: SdkTracerProvider,
    metrics_provider: SdkMeterProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        let results = vec![
            self.logger_provider
                .shutdown()
                .map_err(|e| format!("Logger shutdown failed: {}", e)),
            self.tracer_provider
                .shutdown()
                .map_err(|e| format!("Tracer shutdown failed: {}", e)),
            self.metrics_provider
                .shutdown()
                .map_err(|e| format!("Metrics shutdown failed: {}", e)),
        ];

        let errors: Vec<String> =
            results.into_iter().filter_map(Result::err).collect();
        if errors.is_empty() {
            tracing::info!("OpenTelemetry shutdown completed successfully.");
        } else {
            tracing::error!(
                "OpenTelemetry shutdown encountered errors: {:?}",
                errors
            );
        }
    }
}

pub struct OtelService;

impl OtelService {
    /// Initialize the OpenTelemetry instrumentation in a separate thread.
    pub fn init() -> std::thread::JoinHandle<()> {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                match OtelService.start_instrument() {
                    Ok(otel_guard) => {
                        let _otel_guard = std::sync::Arc::new(otel_guard);
                        tracing::info!("OpenTelemetry instrumentation started in separate thread.");
                        std::future::pending::<()>().await;
                    }
                    Err(e) => {
                        tracing::error!("Failed to start OpenTelemetry instrumentation: {:#}", e);
                    }
                }
            });
        })
    }

    /// Start the OpenTelemetry instrumentation.
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
                KeyValue::new(SERVICE_NAME, CONFIG.package_name),
                KeyValue::new(SERVICE_VERSION, CONFIG.package_version),
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
        .with_endpoint(CONFIG.otel_log_exporter_endpoint)
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
        .with_endpoint(CONFIG.otel_trace_exporter_endpoint)
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
        .with_endpoint(CONFIG.otel_metrics_exporter_endpoint)
        .with_temporality(Temporality::Cumulative)
        .build()?;
    let provider = SdkMeterProvider::builder()
        .with_resource(resource.clone())
        .with_periodic_exporter(exporter)
        .build();

    global::set_meter_provider(provider.clone());

    Ok(provider)
}
