use hyper::header::HOST;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::net::SocketAddr;

fn response(status: StatusCode, body: impl Into<Body>) -> Response<Body> {
    let mut resp = Response::new(body.into());
    *resp.status_mut() = status;
    resp
}

/// 拦截 *.ahin.io 请求，并桥接到后端 DHT 寻址层。
pub async fn handle_wildcard_dns(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let Some(host_value) = req.headers().get(HOST).and_then(|h| h.to_str().ok()) else {
        return Ok(response(StatusCode::BAD_REQUEST, "Missing host header"));
    };

    // host 可能是 `abc.ahin.io:443`，先去掉端口。
    let host_without_port = host_value.split(':').next().unwrap_or(host_value);

    if let Some(domain_prefix) = host_without_port.strip_suffix(".ahin.io") {
        if domain_prefix.len() != 12
            || !domain_prefix
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        {
            return Ok(response(
                StatusCode::BAD_REQUEST,
                "Invalid AHIN domain prefix",
            ));
        }

        println!(
            "🔍 [DNS ROUTER] Incoming web request for {}",
            host_without_port
        );

        // TODO: 生产中这里应该调用 Geo-Spatial DHT + 斩首名单校验。
        println!("🔗 [TUNNEL] Routing traffic to Edge Node via AHIN P2P Mesh.");

        return Ok(response(
            StatusCode::OK,
            format!("Connected to Silicon Entity: {host_without_port}"),
        ));
    }

    Ok(response(StatusCode::NOT_FOUND, "404: Invalid AHIN Domain."))
}

/// 启动一个最小可运行的网关服务。
pub async fn run_dns_dht_bridge(addr: SocketAddr) {
    let make_service =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_wildcard_dns)) });

    let server = Server::bind(&addr).serve(make_service);
    println!("🌐 [BRIDGE] Wildcard DNS bridge listening on http://{addr}");

    if let Err(error) = server.await {
        eprintln!("❌ [BRIDGE] server error: {error}");
    }
}
