use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};

use async_trait::async_trait;
use pingora::{
    http::{RequestHeader, ResponseHeader, StatusCode},
    prelude::{HttpPeer, RoundRobin},
    Result,
};
use pingora_limits::rate::Rate;
use pingora_load_balancing::LoadBalancer;
use pingora_proxy::{ProxyHttp, Session};

pub struct LB(pub Arc<LoadBalancer<RoundRobin>>);

impl LB {
    pub fn get_request_appid(&self, session: &Session) -> Option<String> {
        match session
            .req_header()
            .headers
            .get("appid")
            .map(|appid| appid.to_str())
        {
            None => None,
            Some(appid) => match appid {
                Ok(appid) => Some(appid.to_string()),
                Err(_) => None,
            },
        }
    }
}

static RATE_LIMIT: LazyLock<Rate> = LazyLock::new(|| Rate::new(Duration::from_secs(1)));
static MAX_REQUESTS_PER_SECOND: isize = 1;

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

        let peer = Box::new(HttpPeer::new(upstream, true, "one.one.one.one".to_string()));
        Ok(peer)
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        upstream_request.insert_header("Host", "one.one.one.one")?;
        Ok(())
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        let appid = match self.get_request_appid(session) {
            None => return Ok(true),
            Some(appid) => appid,
        };
        let current_window_request = RATE_LIMIT.observe(&appid, 1);

        if current_window_request > MAX_REQUESTS_PER_SECOND {
            let mut header =
                ResponseHeader::build(StatusCode::TOO_MANY_REQUESTS.as_u16(), None).unwrap();
            header.insert_header("X-Rate-Limit-Limit", MAX_REQUESTS_PER_SECOND.to_string())?;
            header.insert_header("X-Rate-Limit-Remaining", "0")?;
            header.insert_header("X-Rate-Limit-Reset", "1")?;
            session.set_keepalive(None);
            session
                .write_response_header(Box::new(header), true)
                .await?;

            tracing::warn!("Rate limit exceeded for appid: {}", appid);

            return Ok(true);
        }

        Ok(false)
    }
}
