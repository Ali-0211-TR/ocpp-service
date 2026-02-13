//! HTTP request metrics middleware
//!
//! Records `http_requests_total` (counter) and `http_request_duration_seconds` (histogram)
//! for every HTTP request passing through the router.

use axum::{body::Body, extract::MatchedPath, http::Request, middleware::Next, response::Response};
use std::time::Instant;

/// Middleware that records HTTP request metrics:
///
/// - **`http_requests_total`** — counter with labels `method`, `path`, `status`
/// - **`http_request_duration_seconds`** — histogram with labels `method`, `path`
pub async fn http_metrics_middleware(request: Request<Body>, next: Next) -> Response {
    let method = request.method().to_string();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());

    let start = Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed().as_secs_f64();

    let status = response.status().as_u16().to_string();

    metrics::counter!("http_requests_total", "method" => method.clone(), "path" => path.clone(), "status" => status)
        .increment(1);
    metrics::histogram!("http_request_duration_seconds", "method" => method, "path" => path)
        .record(duration);

    response
}
