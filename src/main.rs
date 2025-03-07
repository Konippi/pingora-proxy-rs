use std::time::Duration;

use config::CONFIG;
use load_balancer::LB;
use otel::OtelService;
use pingora::{
    prelude::{TcpHealthCheck, background_service},
    server::Server,
};
use pingora_load_balancing::LoadBalancer;
use pingora_proxy::http_proxy_service;

mod config;
mod load_balancer;
mod otel;

fn main() {
    let mut server = Server::new(None).expect("Failed to create server");
    server.bootstrap();

    let otel_service = background_service("otel", OtelService);
    server.add_service(otel_service);

    let mut upstreams = LoadBalancer::try_from_iter(CONFIG.lb_backends)
        .expect("Failed to create load balancer");
    let health_check = TcpHealthCheck::new();
    upstreams.set_health_check(health_check);
    upstreams.health_check_frequency = Some(Duration::from_secs(1));

    let background = background_service("health check", upstreams);
    let task = background.task();

    server.add_service(background);

    let mut lb = http_proxy_service(&server.configuration, LB(task));
    lb.add_tcp(CONFIG.lb_tcp_listening_endpoint);

    server.add_service(lb);
    server.run_forever();
}
