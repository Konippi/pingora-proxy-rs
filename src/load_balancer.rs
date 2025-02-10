use std::sync::Arc;

use async_trait::async_trait;
use pingora::{
    http::RequestHeader,
    prelude::{HttpPeer, RoundRobin},
    Result,
};
use pingora_load_balancing::LoadBalancer;
use pingora_proxy::{ProxyHttp, Session};

pub struct LB(Arc<LoadBalancer<RoundRobin>>);

impl LB {
    pub fn new(upstreams: &[&str]) -> Self {
        let upstream = LoadBalancer::try_from_iter(upstreams).unwrap();
        Self(Arc::new(upstream))
    }
}

#[async_trait]
impl ProxyHttp for LB {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {
        ()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let upstream = self.0.select(b"", 256).unwrap();

        tracing::info!("upstream peer: {:?}", upstream);

        let peer = HttpPeer::new(upstream, true, "one.one.one.one".to_string());
        Ok(Box::new(peer))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        upstream_request
            .insert_header("Host", "one.one.one.one")
            .unwrap();
        Ok(())
    }
}
