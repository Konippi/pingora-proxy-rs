use opentelemetry::{
    KeyValue,
    global::{self},
    trace::TracerProvider,
};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    Resource,
    logs::SdkLoggerProvider,
    metrics::{SdkMeterProvider, Temporality},
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::{
    SCHEMA_URL,
    resource::{SERVICE_NAME, SERVICE_VERSION},
};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{
    EnvFilter,
    Layer,
    Registry,
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

pub struct OtelService {
    pub fmt_config: FmtConfig,
    pub sampling_rate: f64,
}

impl OtelService {
    pub fn new(config: OtelServiceConfig) -> Self {
        Self {
            fmt_config: config.fmt_config,
            sampling_rate: config.sampling_rate,
        }
    }

    pub fn start_instrument(&self) -> anyhow::Result<OtelGuard> {
        let resource = build_resource();

        let fmt_layer = build_fmt_layer(&self.fmt_config);

        let logger_provider = build_logger_provider(&resource)?;
        let logger_layer = OpenTelemetryTracingBridge::new(&logger_provider).with_filter(
            EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?,
        );

        let tracer_provider = build_tracer_provider(&resource, self.sampling_rate)?;
        let tracer = tracer_provider.tracer(format!("{}-tracer", CONFIG.package_name));
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

pub struct OtelServiceConfig {
    pub fmt_config: FmtConfig,
    pub sampling_rate: f64,
}

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

pub struct FmtConfig {
    pub color: bool,
    pub file: bool,
    pub line_number: bool,
    pub target: bool,
}

fn build_fmt_layer(config: &FmtConfig) -> fmt::Layer<Registry> {
    fmt::Layer::new()
        .with_ansi(config.color)
        .with_file(config.file)
        .with_line_number(config.line_number)
        .with_target(config.target)
}

fn build_logger_provider(resource: &Resource) -> anyhow::Result<SdkLoggerProvider> {
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

// pub struct TracerConfig {
//     pub sampling_rate: f64,
//     pub timeout: Duration,
//     pub max_attributes_per_span: usize,
//     pub max_events_per_span: usize,
//     pub max_queue_size: usize,
//     pub scheduled_delay: Duration,
//     pub max_export_batch_size: usize,
//     pub max_export_timeout: Duration,
// }

fn build_tracer_provider(
    resource: &Resource,
    sampling_rate: f64,
) -> anyhow::Result<SdkTracerProvider> {
    let sampler = Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(sampling_rate)));
    let id_generator = RandomIdGenerator::default();
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(CONFIG.otel_trace_exporter_endpoint.as_str())
        .build()?;
    let provider = SdkTracerProvider::builder()
        .with_sampler(sampler)
        .with_id_generator(id_generator)
        .with_resource(resource.clone())
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(provider.clone());

    Ok(provider)
}

fn build_metrics_provider(resource: &Resource) -> anyhow::Result<SdkMeterProvider> {
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
