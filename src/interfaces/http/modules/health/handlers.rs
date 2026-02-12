//! Health check handler

use std::sync::Arc;
use std::time::Instant;

use axum::{extract::State, http::StatusCode, Json};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde::Serialize;
use utoipa::ToSchema;

use crate::application::SharedSessionRegistry;

/// Health check state
#[derive(Clone)]
pub struct HealthState {
    pub db: DatabaseConnection,
    pub session_registry: SharedSessionRegistry,
    pub started_at: Arc<Instant>,
}

/// Service health response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub database: ComponentHealth,
    pub connected_charge_points: u32,
}

/// Component health status
#[derive(Debug, Serialize, ToSchema)]
pub struct ComponentHealth {
    pub status: String,
    pub latency_ms: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 503, description = "Service is degraded", body = HealthResponse)
    )
)]
pub async fn health_check(
    State(state): State<HealthState>,
) -> (StatusCode, Json<HealthResponse>) {
    let uptime = state.started_at.elapsed().as_secs();
    let connected = state.session_registry.connected_ids().len() as u32;

    // Ping the database
    let db_start = Instant::now();
    let db_health = match state
        .db
        .execute(Statement::from_string(
            state.db.get_database_backend(),
            "SELECT 1".to_string(),
        ))
        .await
    {
        Ok(_) => ComponentHealth {
            status: "ok".to_string(),
            latency_ms: Some(db_start.elapsed().as_millis() as u64),
        },
        Err(_) => ComponentHealth {
            status: "error".to_string(),
            latency_ms: None,
        },
    };

    let overall_status = if db_health.status == "ok" {
        "ok"
    } else {
        "degraded"
    };

    let http_status = if overall_status == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        http_status,
        Json(HealthResponse {
            status: overall_status.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            database: db_health,
            connected_charge_points: connected,
        }),
    )
}
