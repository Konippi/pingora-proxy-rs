use load_balancer::LB;
use pingora::server::Server;
use pingora_proxy::http_proxy_service;

mod load_balancer;
mod logging;

fn main() {
    logging::register_subscriber();

    let mut server = Server::new(None).expect("Failed to create server");
    server.bootstrap();

    let mut lb = http_proxy_service(
        &server.configuration,
        LB::new(&["1.1.1.1:443", "1.0.0.1:443"]),
    );
    lb.add_tcp("0.0.0.0:6188");

    server.add_service(lb);
    server.run_forever();
}
