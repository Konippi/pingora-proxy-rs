use config::CONFIG;
use load_balancer::LB;
use otel::OtelService;
use pingora::{
    prelude::{TcpHealthCheck, background_service},
    server::Server,
    services::Service,
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

    let mut upstreams = LoadBalancer::try_from_iter(CONFIG.lb_backends)
        .expect("Failed to create load balancer");
    let health_check = TcpHealthCheck::new();
    upstreams.set_health_check(health_check);
    upstreams.health_check_frequency = Some(CONFIG.lb_health_check_frequency);

    let health_check_service = background_service("health check", upstreams);
    let health_check_task = health_check_service.task();

    let mut proxy_service =
        http_proxy_service(&server.configuration, LB(health_check_task));
    proxy_service.add_tcp(CONFIG.lb_tcp_listening_endpoint);

    let services: Vec<Box<dyn Service>> = vec![
        Box::new(otel_service),
        Box::new(health_check_service),
        Box::new(proxy_service),
    ];
    server.add_services(services);
    server.run_forever();
}
