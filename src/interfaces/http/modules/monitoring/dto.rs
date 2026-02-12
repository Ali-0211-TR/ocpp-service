//! Monitoring DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HeartbeatStatusDto {
    pub charge_point_id: String,
    pub last_heartbeat: Option<String>,
    pub last_seen: Option<String>,
    pub is_connected: bool,
    pub status: String,
    pub seconds_since_heartbeat: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConnectionStatsDto {
    pub total: usize,
    pub online: usize,
    pub offline: usize,
    pub stale: usize,
}
