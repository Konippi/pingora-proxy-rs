use std::time::Duration;

use load_balancer::LB;
use otel::{FmtConfig, OtelService, OtelServiceConfig};
use pingora::{
    prelude::{TcpHealthCheck, background_service},
    server::Server,
};
use pingora_load_balancing::LoadBalancer;
use pingora_proxy::http_proxy_service;
use supports_color::Stream;

mod config;
mod load_balancer;
mod otel;

fn main() {
    // Setup OpenTelemetry
    let otel_service = OtelService::new(OtelServiceConfig {
        fmt_config: FmtConfig {
            color: supports_color::on(Stream::Stdout).is_some(),
            file: true,
            line_number: true,
            target: true,
        },
    });
    let _otel = otel_service
        .start_instrument()
        .expect("Failed to initialize OpenTelemetry");

    // Setup Pingora Server
    let mut server = Server::new(None).expect("Failed to create server");
    server.bootstrap();

    let mut upstreams = LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443"]).unwrap();
    let health_check = TcpHealthCheck::new();
    upstreams.set_health_check(health_check);
    upstreams.health_check_frequency = Some(Duration::from_secs(1));

    let background = background_service("health check", upstreams);
    let task = background.task();

    server.add_service(background);

    let mut lb = http_proxy_service(&server.configuration, LB(task));
    lb.add_tcp("0.0.0.0:6188");

    server.add_service(lb);
    server.run_forever();
}
