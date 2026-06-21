//! Local HTTP proxy for stream URLs.
//!
//! YouTube/Invidious stream URLs often require specific headers (User-Agent,
//! Referer, Range) and may be rejected by an Android WebView. The WebView
//! instead requests this localhost proxy, which forwards the request with the
//! headers the stream provider expects and pipes the response back.

use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::Query,
    http::{HeaderMap, HeaderName, HeaderValue, Request, Response, StatusCode},
    routing::get,
    Router,
};
use serde::Deserialize;
use tokio::net::TcpListener;

#[derive(Deserialize)]
struct ProxyQuery {
    url: String,
}

/// Headers that should not be blindly forwarded between client and upstream.
const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

fn is_hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP_HEADERS.contains(&name.to_ascii_lowercase().as_str())
}

fn copy_request_headers(forwarded: &HeaderMap, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let mut out = builder;
    for name in ["range", "accept", "accept-encoding", "accept-language"] {
        if let Some(value) = forwarded.get(name) {
            if let Ok(header_name) = HeaderName::from_bytes(name.as_bytes()) {
                if let Ok(header_value) = HeaderValue::from_bytes(value.as_bytes()) {
                    out = out.header(header_name, header_value);
                }
            }
        }
    }
    out
}

fn build_response(upstream: &reqwest::Response) -> Response<Body> {
    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);

    let mut builder = Response::builder().status(status);
    for (key, value) in upstream.headers() {
        if is_hop_by_hop(key.as_str()) {
            continue;
        }
        if let Ok(name) = HeaderName::from_bytes(key.as_str().as_bytes()) {
            if let Ok(val) = HeaderValue::from_bytes(value.as_bytes()) {
                builder = builder.header(name, val);
            }
        }
    }

    builder.body(Body::empty()).unwrap_or_else(|_| {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::empty())
            .unwrap()
    })
}

async fn proxy_handler(
    Query(query): Query<ProxyQuery>,
    headers: HeaderMap,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    let target = urlencoding::decode(&query.url).map_err(|_| StatusCode::BAD_REQUEST)?;

    let client = reqwest::Client::new();
    let mut upstream = client.request(req.method().clone(), target.as_ref());
    upstream = copy_request_headers(&headers, upstream);

    // Headers that make YouTube/Invidious stream URLs playable from a generic
    // HTTP client.
    upstream = upstream.header(
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    );
    upstream = upstream.header("referer", "https://www.youtube.com/");

    let upstream_resp = upstream.send().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    let mut response = build_response(&upstream_resp);

    let stream = upstream_resp.bytes_stream();
    *response.body_mut() = Body::from_stream(stream);
    Ok(response)
}

pub struct StreamProxy {
    pub base_url: String,
}

impl StreamProxy {
    pub async fn start() -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr: SocketAddr = listener.local_addr()?;
        let base_url = format!("http://{addr}");

        let app = Router::new().route("/proxy", get(proxy_handler));

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("stream proxy server error: {e}");
            }
        });

        Ok(Self { base_url })
    }

    pub fn proxied_url(&self, original: &str) -> String {
        format!("{}/proxy?url={}", self.base_url, urlencoding::encode(original))
    }
}
