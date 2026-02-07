//! Charge Point API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::dto::{ApiResponse, ChargePointDto, ChargePointStats};
use crate::infrastructure::Storage;
use crate::session::SharedSessionManager;

/// Application state for API handlers
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub session_manager: SharedSessionManager,
}

/// Get all charge points
#[utoipa::path(
    get,
    path = "/api/v1/charge-points",
    tag = "Charge Points",
    responses(
        (status = 200, description = "List of charge points", body = ApiResponse<Vec<ChargePointDto>>)
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
)]
pub async fn list_charge_points(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ChargePointDto>>>, (StatusCode, Json<ApiResponse<Vec<ChargePointDto>>>)>
{
    match state.storage.list_charge_points().await {
        Ok(charge_points) => {
            let dtos: Vec<ChargePointDto> = charge_points
                .into_iter()
                .map(|cp| {
                    let is_online = state.session_manager.is_connected(&cp.id);
                    ChargePointDto::from_domain(cp, is_online)
                })
                .collect();

            Ok(Json(ApiResponse::success(dtos)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Get charge point by ID
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}",
    tag = "Charge Points",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    responses(
        (status = 200, description = "Charge point details", body = ApiResponse<ChargePointDto>),
        (status = 404, description = "Charge point not found")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
)]
pub async fn get_charge_point(
    State(state): State<AppState>,
    Path(charge_point_id): Path<String>,
) -> Result<Json<ApiResponse<ChargePointDto>>, (StatusCode, Json<ApiResponse<ChargePointDto>>)> {
    println!("Fetching charge point with ID: {}", charge_point_id);
    match state.storage.get_charge_point(&charge_point_id).await {
        Ok(Some(cp)) => {
            let is_online = state.session_manager.is_connected(&cp.id);
            Ok(Json(ApiResponse::success(ChargePointDto::from_domain(
                cp, is_online,
            ))))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("Charge point '{}' not found", charge_point_id))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Delete charge point
#[utoipa::path(
    delete,
    path = "/api/v1/charge-points/{charge_point_id}",
    tag = "Charge Points",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    responses(
        (status = 200, description = "Charge point deleted"),
        (status = 404, description = "Charge point not found")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
)]
pub async fn delete_charge_point(
    State(state): State<AppState>,
    Path(charge_point_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.storage.delete_charge_point(&charge_point_id).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Get charge point statistics
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/stats",
    tag = "Charge Points",
    responses(
        (status = 200, description = "Charge point statistics", body = ApiResponse<ChargePointStats>)
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
)]
pub async fn get_charge_point_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ChargePointStats>>, (StatusCode, Json<ApiResponse<ChargePointStats>>)>
{
    match state.storage.list_charge_points().await {
        Ok(charge_points) => {
            let total = charge_points.len() as u32;
            let mut online = 0u32;
            let mut charging = 0u32;

            for cp in &charge_points {
                if state.session_manager.is_connected(&cp.id) {
                    online += 1;

                    // Check if any connector is charging
                    for conn in &cp.connectors {
                        if conn.status == crate::domain::ConnectorStatus::Charging {
                            charging += 1;
                            break;
                        }
                    }
                }
            }

            let stats = ChargePointStats {
                total,
                online,
                offline: total - online,
                charging,
            };

            Ok(Json(ApiResponse::success(stats)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Get online charge point IDs
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/online",
    tag = "Charge Points",
    responses(
        (status = 200, description = "List of online charge point IDs", body = ApiResponse<Vec<String>>)
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
)]
pub async fn get_online_charge_points(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<String>>> {
    let online_ids = state.session_manager.connected_ids();
    Json(ApiResponse::success(online_ids))
}
