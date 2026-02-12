//! Monitoring API handlers

use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use super::dto::{ConnectionStatsDto, HeartbeatStatusDto};
use crate::application::HeartbeatMonitor;
use crate::interfaces::http::common::ApiResponse;

/// Monitoring state
#[derive(Clone)]
pub struct MonitoringState {
    pub heartbeat_monitor: Arc<HeartbeatMonitor>,
}

#[utoipa::path(
    get,
    path = "/api/v1/monitoring/heartbeats",
    responses(
        (status = 200, description = "Heartbeat statuses", body = ApiResponse<Vec<HeartbeatStatusDto>>)
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
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

#[utoipa::path(
    get,
    path = "/api/v1/monitoring/stats",
    responses(
        (status = 200, description = "Connection stats", body = ApiResponse<ConnectionStatsDto>)
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
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

#[utoipa::path(
    get,
    path = "/api/v1/monitoring/online",
    responses(
        (status = 200, description = "Online charge point IDs", body = ApiResponse<Vec<String>>)
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Monitoring"
)]
pub async fn get_online_charge_points(
    State(state): State<MonitoringState>,
) -> Json<ApiResponse<Vec<String>>> {
    let online = state.heartbeat_monitor.get_online_charge_points().await;
    Json(ApiResponse::success(online))
}
