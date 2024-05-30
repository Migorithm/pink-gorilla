pub mod conversions;
pub mod tri_node;

use async_trait::async_trait;
use conversions::headers::BearerToken;
use pingora::prelude::*;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tri_node::Trie;

macro_rules! add_loadbalanced_service {
    (   $server:ident,
        $mapper:ident,
        $(
            $prefix: literal=> $addr:literal
        ),*
    ) => {

        $(
        let mut service =  LoadBalancer::try_from_iter([$addr]).unwrap();
        // adding a basic health check service. So all requests succeed and the broken peer is never used.
        // if that peer were to become healthy again, it would be re-included in the round robin again in within 1 second.
        let hc = TcpHealthCheck::new();
        service.set_health_check(hc);
        service.health_check_frequency = Some(std::time::Duration::from_secs(1));
        let background = background_service("health check", service);
        let upstreams = background.task();
        $server.add_service(background);
        $mapper.insert($prefix, upstreams);
        )*
    };
}

fn main() {
    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    let mut service_mapper = HashMap::new();

    add_loadbalanced_service!(server,service_mapper,
        "/auth" => "0.0.0.0:44410",
        "/task" => "0.0.0.0:44447"
    );

    let mut path_mapper = Trie::default();
    path_mapper.insert("/auth");
    path_mapper.insert("/task");

    // put lb instance to a proxy service via http_proxy_service
    let mut lb = http_proxy_service(
        &server.configuration,
        LB {
            service_mapper,
            path_mapper,
        },
    );
    lb.add_tcp("0.0.0.0:44444");
    server.add_service(lb);
    server.run_forever();
}

// ? how do I customize balancing logic?
pub struct LB {
    service_mapper: HashMap<&'static str, Arc<LoadBalancer<RoundRobin>>>,
    path_mapper: Trie,
}

#[async_trait]
impl ProxyHttp for LB {
    // ? how do I customize the upstream peer selection?
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    /// returns the address where the request should be forwarded
    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        // ! Room for optimization: use a trie to store the prefixes
        let path = session.req_header().uri.path();
        let matched_path = self.path_mapper.search(path);
        let Some(path) = matched_path else {
            return Err(Error::explain(HTTPStatus(404), "No matching path found"));
        };
        let addr = self
            .service_mapper
            .get(path.as_str())
            .unwrap()
            .select(b"", 256)
            .unwrap();

        println!("found upstream peer...{:?}", addr);

        let mut peer = Box::new(HttpPeer::new(addr, false, "".to_string()));
        peer.options.connection_timeout = Some(Duration::from_millis(100));
        Ok(peer)
    }

    /// Modify the request before it is sent to the upstream
    /// Unlike [Self::request_filter()], this filter allows to change the request headers to send
    /// to the upstream.
    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        upstream_request
            .insert_header("Host", "one.one.one.one")
            .unwrap();

        Ok(())
    }

    // ! This gets request earlier than [Self::upstream_request_filter()]
    /// Handle the incoming request.
    ///
    /// In this phase, users can parse, validate, rate limit, perform access control and/or
    /// return a response for this request.
    ///
    /// If the user already sent a response to this request, an `Ok(true)` should be returned so that
    /// the proxy would exit. The proxy continues to the next phases when `Ok(false)` is returned.
    ///
    /// By default this filter does nothing and returns `Ok(false)`.
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        let req_header = session.req_header_mut();
        if req_header.uri.path().starts_with("/auth") {
            // ! Errorcase: if there is no Authorication Bearer header, return a 401
            let bearer_token: BearerToken = (&req_header.headers).try_into()?;
            let user_id = bearer_token.get_user_id()?;
            req_header.insert_header("X-USER-ID", user_id).unwrap();
        };

        Ok(false)
    }
}
