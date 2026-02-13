//! Request ID middleware
//!
//! Generates a unique `X-Request-Id` UUID for every HTTP request,
//! propagates it into a `tracing::Span` so all downstream logs carry the ID,
//! and echoes it back in the response header.

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use uuid::Uuid;

/// Header name for the request correlation ID.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Middleware that assigns (or propagates) `X-Request-Id`.
///
/// 1. If the incoming request already contains `X-Request-Id`, reuse it.
/// 2. Otherwise, generate a new UUID v4.
/// 3. Store the ID in request extensions (available to handlers).
/// 4. Create a `tracing::info_span!("request", request_id = ...)` so every
///    log line emitted while processing this request carries the ID.
/// 5. Echo `X-Request-Id` in the response headers.
pub async fn request_id_middleware(mut request: Request<Body>, next: Next) -> Response {
    // Reuse existing header or generate a new one
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Store in extensions so handlers can access it via `Extension<RequestId>`
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    // Build a tracing span that wraps the entire request processing
    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
    );

    let _guard = span.enter();

    let mut response = next.run(request).await;

    // Echo back in the response
    if let Ok(value) = request_id.parse() {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }

    response
}

/// New-type wrapper for the request ID, stored in request extensions.
///
/// Extract in handlers: `Extension(RequestId(id)): Extension<RequestId>`
#[derive(Clone, Debug)]
pub struct RequestId(pub String);
