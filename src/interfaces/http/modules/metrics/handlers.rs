//! Prometheus metrics handler
//!
//! Exposes `GET /metrics` returning Prometheus text format.
//! The handler reads from the global `metrics-exporter-prometheus` recorder.

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use metrics_exporter_prometheus::PrometheusHandle;

/// Shared state for the metrics endpoint
#[derive(Clone)]
pub struct MetricsState {
    pub handle: PrometheusHandle,
}

/// `GET /metrics` — Prometheus scrape endpoint (no auth)
pub async fn prometheus_metrics(State(state): State<MetricsState>) -> impl IntoResponse {
    let body = state.handle.render();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}
