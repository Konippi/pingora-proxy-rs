use opentelemetry::{
    global::{self},
    KeyValue,
};
use opentelemetry_otlp::{MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    metrics::{SdkMeterProvider, Temporality},
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
    Resource,
};
use opentelemetry_semantic_conventions::{
    resource::{SERVICE_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use tracing_opentelemetry::MetricsLayer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{util::SubscriberInitExt, Registry};

use crate::config::CONFIG;

struct OtelGuard {
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
    }
}

pub fn init(sampling_rate: f64) -> anyhow::Result<()> {
    let resource = build_resource();

    let tracer_provider = build_tracer_provider(&resource, sampling_rate)?;
    let tracer = global::tracer("pingora-proxy-tracer");
    let tracer_layer = OpenTelemetryLayer::new(tracer);

    let metrics_provider = build_metrics_provider(&resource)?;
    let metrics = global::meter("pingora-proxy-metrics");
    let metrics_layer = MetricsLayer::new(metrics_provider);

    Registry::default().init();

    OtelGuard {
        tracer_provider,
        metrics_provider,
    };

    Ok(())
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
