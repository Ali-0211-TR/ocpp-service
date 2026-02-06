//! Monitoring API handlers
//!
//! Provides endpoints for monitoring charge point status and heartbeats.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::ApiResponse;
use crate::application::services::HeartbeatMonitor;

/// Monitoring state
#[derive(Clone)]
pub struct MonitoringState {
    pub heartbeat_monitor: Arc<HeartbeatMonitor>,
}

/// Heartbeat status DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HeartbeatStatusDto {
    pub charge_point_id: String,
    #[schema(example = "2026-02-06T10:30:00Z")]
    pub last_heartbeat: Option<String>,
    #[schema(example = "2026-02-06T10:30:05Z")]
    pub last_seen: Option<String>,
    pub is_connected: bool,
    #[schema(example = "Online")]
    pub status: String,
    #[schema(example = 30)]
    pub seconds_since_heartbeat: Option<i64>,
}

/// Connection stats DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConnectionStatsDto {
    pub total: usize,
    pub online: usize,
    pub offline: usize,
    pub stale: usize,
}

/// Get heartbeat status for all charge points
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/heartbeats",
    responses(
        (status = 200, description = "List of heartbeat statuses", body = ApiResponse<Vec<HeartbeatStatusDto>>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
    tag = "Monitoring"
)]
pub async fn get_heartbeat_statuses(
    State(state): State<MonitoringState>,
) -> Json<ApiResponse<Vec<HeartbeatStatusDto>>> {
    match state.heartbeat_monitor.get_all_statuses().await {
        Ok(statuses) => {
            let dtos: Vec<HeartbeatStatusDto> = statuses
                .into_iter()
                .map(|s| HeartbeatStatusDto {
                    charge_point_id: s.charge_point_id,
                    last_heartbeat: s.last_heartbeat.map(|dt| dt.to_rfc3339()),
                    last_seen: s.last_seen.map(|dt| dt.to_rfc3339()),
                    is_connected: s.is_connected,
                    status: s.status.to_string(),
                    seconds_since_heartbeat: s.seconds_since_heartbeat,
                })
                .collect();
            Json(ApiResponse::success(dtos))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

/// Get connection statistics
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/stats",
    responses(
        (status = 200, description = "Connection statistics", body = ApiResponse<ConnectionStatsDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
    tag = "Monitoring"
)]
pub async fn get_connection_stats(
    State(state): State<MonitoringState>,
) -> Json<ApiResponse<ConnectionStatsDto>> {
    match state.heartbeat_monitor.get_connection_stats().await {
        Ok(stats) => {
            let dto = ConnectionStatsDto {
                total: stats.total,
                online: stats.online,
                offline: stats.offline,
                stale: stats.stale,
            };
            Json(ApiResponse::success(dto))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

/// Get list of currently online charge points
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/online",
    responses(
        (status = 200, description = "List of online charge point IDs", body = ApiResponse<Vec<String>>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
    tag = "Monitoring"
)]
pub async fn get_online_charge_points(
    State(state): State<MonitoringState>,
) -> Json<ApiResponse<Vec<String>>> {
    let online = state.heartbeat_monitor.get_online_charge_points().await;
    Json(ApiResponse::success(online))
}
