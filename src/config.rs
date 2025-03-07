use std::{sync::LazyLock, time::Duration};

use supports_color::Stream;

#[derive(Debug)]
pub struct Config {
    // package
    pub package_name: &'static str,
    pub package_version: &'static str,

    // load-balancer
    pub lb_backends: [&'static str; 2],
    pub lb_tcp_listening_endpoint: &'static str,

    // tracing-subscriber
    pub tracing_subscriber_fmt_color: bool,
    pub tracing_subscriber_fmt_file: bool,
    pub tracing_subscriber_fmt_line_number: bool,
    pub tracing_subscriber_fmt_target: bool,
    pub tracing_subscriber_fmt_thread_names: bool,

    // opentelemetry
    pub otel_log_exporter_endpoint: &'static str,
    pub otel_trace_processor_max_queue_size: usize,
    pub otel_trace_processor_scheduled_delay: Duration,
    pub otel_trace_processor_max_export_batch_size: usize,
    pub otel_trace_processor_max_export_timeout: Duration,
    pub otel_trace_processor_max_concurrent_exports: usize,
    pub otel_trace_exporter_endpoint: &'static str,
    pub otel_metrics_exporter_endpoint: &'static str,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    // package
    package_name: env!("CARGO_PKG_NAME"),
    package_version: env!("CARGO_PKG_VERSION"),

    // load-balancer
    lb_backends: ["1.1.1.1:443", "1.0.0.1:443"],
    lb_tcp_listening_endpoint: "0.0.0.0:6188",

    // tracing-subscriber
    tracing_subscriber_fmt_color: supports_color::on(Stream::Stdout).is_some(),
    tracing_subscriber_fmt_file: true,
    tracing_subscriber_fmt_line_number: true,
    tracing_subscriber_fmt_target: true,
    tracing_subscriber_fmt_thread_names: true,

    // opentelemetry
    otel_log_exporter_endpoint: "http://localhost:4317",
    otel_trace_processor_max_queue_size: 2048,
    otel_trace_processor_scheduled_delay: Duration::from_millis(1000),
    otel_trace_processor_max_export_batch_size: 512,
    otel_trace_processor_max_export_timeout: Duration::from_millis(30000),
    otel_trace_processor_max_concurrent_exports: 32,
    otel_trace_exporter_endpoint: "http://localhost:4317",
    otel_metrics_exporter_endpoint: "http://localhost:4317",
});
