use std::time::Duration;

use load_balancer::LB;
use pingora::{
    prelude::{background_service, TcpHealthCheck},
    server::Server,
};
use pingora_load_balancing::LoadBalancer;
use pingora_proxy::http_proxy_service;

mod load_balancer;
mod logging;

fn main() {
    logging::register_subscriber();

    let mut server = Server::new(None).expect("Failed to create server");
    server.bootstrap();

    let mut upstreams = LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443"]).unwrap();
    let health_check = TcpHealthCheck::new();
    upstreams.set_health_check(health_check);
    upstreams.health_check_frequency = Some(Duration::from_secs(1));

    let background = background_service("health check", upstreams);
    let task = background.task();

    let mut lb = http_proxy_service(&server.configuration, LB(task));
    lb.add_tcp("0.0.0.0:6188");

    server.add_service(lb);
    server.run_forever();
}
