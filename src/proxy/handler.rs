use crate::app::Mapping;
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tokio::sync::watch;

/// Headers that must not be forwarded between hops (RFC 2616 §13.5.1).
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

fn full_body(s: &str) -> BoxBody {
    Full::new(Bytes::from(s.to_string()))
        .map_err(|never| match never {})
        .boxed()
}

/// Handle an incoming request by routing based on the Host header.
pub async fn handle_request(
    req: Request<Incoming>,
    mappings_rx: watch::Receiver<Vec<Mapping>>,
) -> Result<Response<BoxBody>, hyper::Error> {
    // Extract host from the Host header
    let host = req
        .headers()
        .get(hyper::header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|h| {
            // Strip port from Host header if present (e.g., "my-project.localhost:80")
            h.split(':').next().unwrap_or(h).to_lowercase()
        });

    let host = match host {
        Some(h) => h,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(full_body("Missing Host header"))
                .unwrap());
        }
    };

    // Look up the mapping
    let mappings = mappings_rx.borrow().clone();
    let mapping = mappings.iter().find(|m| m.domain == host);

    let port = match mapping {
        Some(m) => m.port,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(full_body(&format!("No mapping found for host: {}", host)))
                .unwrap());
        }
    };

    // Build the forwarding URI
    let uri_str = format!(
        "http://127.0.0.1:{}{}",
        port,
        req.uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/")
    );

    let uri: hyper::Uri = match uri_str.parse() {
        Ok(u) => u,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(full_body("Invalid URI"))
                .unwrap());
        }
    };

    // Build the forwarded request, stripping hop-by-hop headers
    let method = req.method().clone();
    let mut builder = Request::builder().method(method).uri(uri);

    for (key, value) in req.headers() {
        let name = key.as_str().to_lowercase();
        if !HOP_BY_HOP.contains(&name.as_str()) {
            builder = builder.header(key.clone(), value.clone());
        }
    }

    let forwarded_req = builder
        .body(req.into_body())
        .expect("failed to build forwarded request");

    // Send the request to the target server
    let client: Client<_, Incoming> =
        Client::builder(TokioExecutor::new()).build_http();

    match client.request(forwarded_req).await {
        Ok(resp) => {
            // Strip hop-by-hop headers from response
            let (parts, body) = resp.into_parts();
            let mut builder = Response::builder().status(parts.status);
            for (key, value) in &parts.headers {
                let name = key.as_str().to_lowercase();
                if !HOP_BY_HOP.contains(&name.as_str()) {
                    builder = builder.header(key.clone(), value.clone());
                }
            }
            Ok(builder
                .body(body.map_err(|e| e).boxed())
                .unwrap())
        }
        Err(e) => Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(full_body(&format!(
                "Failed to connect to 127.0.0.1:{} — {}",
                port, e
            )))
            .unwrap()),
    }
}
