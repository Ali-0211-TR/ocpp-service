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

/// Статус heartbeat зарядной станции
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HeartbeatStatusDto {
    /// ID зарядной станции
    pub charge_point_id: String,
    /// Время последнего heartbeat-сообщения (ISO 8601)
    #[schema(example = "2026-02-06T10:30:00Z")]
    pub last_heartbeat: Option<String>,
    /// Время последней активности (любое сообщение, ISO 8601)
    #[schema(example = "2026-02-06T10:30:05Z")]
    pub last_seen: Option<String>,
    /// Подключена ли станция по WebSocket сейчас
    pub is_connected: bool,
    /// Статус: `Online`, `Offline`, `Stale` (нет heartbeat дольше порога)
    #[schema(example = "Online")]
    pub status: String,
    /// Секунд с последнего heartbeat
    #[schema(example = 30)]
    pub seconds_since_heartbeat: Option<i64>,
}

/// Статистика подключений
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConnectionStatsDto {
    /// Общее количество известных станций
    pub total: usize,
    /// Подключенные сейчас
    pub online: usize,
    /// Отключенные
    pub offline: usize,
    /// Подключены, но не присылают heartbeat в срок
    pub stale: usize,
}

/// Статусы heartbeat всех станций
///
/// Возвращает время последнего heartbeat, статус соединения
/// и время с последней активности для каждой станции.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/heartbeats",
    responses(
        (status = 200, description = "Список статусов heartbeat", body = ApiResponse<Vec<HeartbeatStatusDto>>),
        (status = 401, description = "Не авторизован")
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

/// Статистика подключений
///
/// Общая сводка: total, online, offline, stale.
/// Используйте для виджета мониторинга на дашборде.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/stats",
    responses(
        (status = 200, description = "Статистика подключений", body = ApiResponse<ConnectionStatsDto>),
        (status = 401, description = "Не авторизован")
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

/// Список онлайн-станций (мониторинг)
///
/// Возвращает ID станций, которые сейчас подключены по WebSocket.
#[utoipa::path(
    get,
    path = "/api/v1/monitoring/online",
    responses(
        (status = 200, description = "Массив ID онлайн-станций", body = ApiResponse<Vec<String>>),
        (status = 401, description = "Не авторизован")
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
