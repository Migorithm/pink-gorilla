use async_trait::async_trait;
use pingora::prelude::*;
use std::{ops::Deref, sync::Arc};

fn main() {
    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    // ? how do I make it dymamic when an upstream is added or removed?
    let mut upstreams =
        LoadBalancer::try_from_iter(["1.1.1.1:443", "1.0.0.1:443", "127.0.0.1:343"]).unwrap();

    // adding a basic health check service. So all requests succeed and the broken peer is never used.
    // if that peer were to become healthy again, it would be re-included in the round robin again in within 1 second.
    let hc = TcpHealthCheck::new();

    upstreams.set_health_check(hc);
    upstreams.health_check_frequency = Some(std::time::Duration::from_secs(1));
    let background = background_service("health check", upstreams);
    let upstreams = background.task();

    // put lb instance to a proxy service via http_proxy_service
    let mut lb = http_proxy_service(&my_server.configuration, LB(upstreams));

    lb.add_tcp("0.0.0.0:44444");

    my_server.add_service(background);
    my_server.add_service(lb);

    my_server.run_forever();
}

// ? how do I customize balancing logic?
pub struct LB(Arc<LoadBalancer<RoundRobin>>);
impl Deref for LB {
    type Target = Arc<LoadBalancer<RoundRobin>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl ProxyHttp for LB {
    // ? how do I customize the upstream peer selection?
    type CTX = ();
    fn new_ctx(&self) {}

    /// returns the address where the request should be forwarded
    async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut ()) -> Result<Box<HttpPeer>> {
        let upstream = self
            .select(b"", 256) // hash doesn't matter for round robin
            .unwrap();

        println!("upstream peer is: {upstream:?}");

        // Set SNI to one.one.one.one
        let peer = Box::new(HttpPeer::new(upstream, true, "one.one.one.one".to_string()));
        Ok(peer)
    }

    // In order for the 1.1.1.1 backends to accept our requests,
    // a host header must be present.
    // Adding this header can be done by the upstream_request_filter() callback which modifies the request
    // header after the connection to the backends are established
    // and before the request header is sent.
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
