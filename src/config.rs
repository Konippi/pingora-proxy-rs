use std::{sync::LazyLock, time::Duration};

#[derive(Debug)]
pub struct Config {
    // package
    pub package_name: String,
    pub package_version: String,

    // opentelemetry
    pub otel_trace_processor_max_queue_size: usize,
    pub otel_trace_processor_scheduled_delay: Duration,
    pub otel_trace_processor_max_export_batch_size: usize,
    pub otel_trace_processor_max_export_timeout: Duration,
    pub otel_trace_processor_max_concurrent_exports: usize,
    pub otel_trace_exporter_endpoint: String,
    pub otel_metrics_exporter_endpoint: String,
    pub otel_log_exporter_endpoint: String,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    // package
    package_name: env!("CARGO_PKG_NAME").to_string(),
    package_version: env!("CARGO_PKG_VERSION").to_string(),

    // opentelemetry
    otel_trace_processor_max_queue_size: 2048,
    otel_trace_processor_scheduled_delay: Duration::from_millis(1000),
    otel_trace_processor_max_export_batch_size: 512,
    otel_trace_processor_max_export_timeout: Duration::from_millis(30000),
    otel_trace_processor_max_concurrent_exports: 32,
    otel_trace_exporter_endpoint: "http://localhost:4317".to_string(),
    otel_metrics_exporter_endpoint: "http://localhost:4317".to_string(),
    otel_log_exporter_endpoint: "http://localhost:4317".to_string(),
});
