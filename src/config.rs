use std::sync::LazyLock;

pub struct Config {
    // package
    pub package_name: String,
    pub package_version: String,

    // opentelemetry
    pub otel_trace_exporter_endpoint: String,
    pub otel_metrics_exporter_endpoint: String,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    package_name: env!("CARGO_PKG_NAME").to_string(),
    package_version: env!("CARGO_PKG_VERSION").to_string(),
    otel_trace_exporter_endpoint: "http://localhost:4317".to_string(),
    otel_metrics_exporter_endpoint: "http://localhost:4317".to_string(),
});
