//! Charge Point API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use super::dto::{ChargePointDto, ChargePointStats, ConnectorDto, CreateChargePointRequest, CreateConnectorRequest, SetPasswordRequest, SetPasswordResponse};
use crate::application::SharedSessionRegistry;
use crate::domain::RepositoryProvider;
use crate::interfaces::http::common::ApiResponse;

use crate::infrastructure::crypto::password::hash_password;
use crate::interfaces::http::common::ValidatedJson;

/// Charge-point handler state
#[derive(Clone)]
pub struct AppState {
    pub repos: Arc<dyn RepositoryProvider>,
    pub session_registry: SharedSessionRegistry,
}

#[utoipa::path(
    get,
    path = "/api/v1/charge-points",
    tag = "Charge Points",
    responses(
        (status = 200, description = "List of charge points", body = ApiResponse<Vec<ChargePointDto>>)
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn list_charge_points(
    State(state): State<AppState>,
) -> Result<
    Json<ApiResponse<Vec<ChargePointDto>>>,
    (StatusCode, Json<ApiResponse<Vec<ChargePointDto>>>),
> {
    match state.repos.charge_points().find_all().await {
        Ok(charge_points) => {
            let dtos: Vec<ChargePointDto> = charge_points
                .into_iter()
                .map(|cp| {
                    let is_online = state.session_registry.is_connected(&cp.id);
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

#[utoipa::path(
    post,
    path = "/api/v1/charge-points",
    tag = "Charge Points",
    request_body = CreateChargePointRequest,
    responses(
        (status = 201, description = "Charge point created", body = ApiResponse<ChargePointDto>),
        (status = 409, description = "Already exists"),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn create_charge_point(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateChargePointRequest>,
) -> Result<
    (StatusCode, Json<ApiResponse<ChargePointDto>>),
    (StatusCode, Json<ApiResponse<ChargePointDto>>),
> {
    use crate::domain::ChargePoint;

    // Check if already exists
    match state.repos.charge_points().find_by_id(&body.id).await {
        Ok(Some(_)) => {
            return Err((
                StatusCode::CONFLICT,
                Json(ApiResponse::error(format!(
                    "Charge point '{}' already exists",
                    body.id
                ))),
            ));
        }
        Ok(None) => {}
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            ));
        }
    }

    // Create domain entity
    let mut cp = ChargePoint::new(&body.id);
    cp.vendor = body.vendor;
    cp.model = body.model;
    cp.serial_number = body.serial_number;

    // Auto-create connectors (1-based)
    for i in 1..=body.num_connectors {
        cp.add_connector(i);
    }

    // Save to repository
    if let Err(e) = state.repos.charge_points().save(cp.clone()).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ));
    }

    let is_online = state.session_registry.is_connected(&cp.id);
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(ChargePointDto::from_domain(cp, is_online))),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}",
    tag = "Charge Points",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    responses(
        (status = 200, description = "Charge point details", body = ApiResponse<ChargePointDto>),
        (status = 404, description = "Not found")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn get_charge_point(
    State(state): State<AppState>,
    Path(charge_point_id): Path<String>,
) -> Result<Json<ApiResponse<ChargePointDto>>, (StatusCode, Json<ApiResponse<ChargePointDto>>)> {
    match state.repos.charge_points().find_by_id(&charge_point_id).await {
        Ok(Some(cp)) => {
            let is_online = state.session_registry.is_connected(&cp.id);
            Ok(Json(ApiResponse::success(ChargePointDto::from_domain(
                cp, is_online,
            ))))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' not found",
                charge_point_id
            ))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/charge-points/{charge_point_id}",
    tag = "Charge Points",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn delete_charge_point(
    State(state): State<AppState>,
    Path(charge_point_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.repos.charge_points().delete(&charge_point_id).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/charge-points/stats",
    tag = "Charge Points",
    responses(
        (status = 200, description = "Stats", body = ApiResponse<ChargePointStats>)
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn get_charge_point_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ChargePointStats>>, (StatusCode, Json<ApiResponse<ChargePointStats>>)>
{
    match state.repos.charge_points().find_all().await {
        Ok(charge_points) => {
            let total = charge_points.len() as u32;
            let mut online = 0u32;
            let mut charging = 0u32;

            for cp in &charge_points {
                if state.session_registry.is_connected(&cp.id) {
                    online += 1;
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

#[utoipa::path(
    get,
    path = "/api/v1/charge-points/online",
    tag = "Charge Points",
    responses(
        (status = 200, description = "Online IDs", body = ApiResponse<Vec<String>>)
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn get_online_charge_points(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<String>>> {
    let online_ids = state.session_registry.connected_ids();
    Json(ApiResponse::success(online_ids))
}

// ── Connector endpoints ────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/connectors",
    tag = "Connectors",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    responses(
        (status = 200, description = "List of connectors", body = ApiResponse<Vec<ConnectorDto>>),
        (status = 404, description = "Not found")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn list_connectors(
    State(state): State<AppState>,
    Path(charge_point_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<ConnectorDto>>>, (StatusCode, Json<ApiResponse<Vec<ConnectorDto>>>)>
{
    match state.repos.charge_points().find_by_id(&charge_point_id).await {
        Ok(Some(cp)) => {
            let connectors: Vec<ConnectorDto> = cp
                .connectors
                .into_iter()
                .map(ConnectorDto::from_domain)
                .collect();
            Ok(Json(ApiResponse::success(connectors)))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' not found",
                charge_point_id
            ))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/connectors/{connector_id}",
    tag = "Connectors",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID"),
        ("connector_id" = u32, Path, description = "Connector number")
    ),
    responses(
        (status = 200, description = "Connector details", body = ApiResponse<ConnectorDto>),
        (status = 404, description = "Not found")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn get_connector(
    State(state): State<AppState>,
    Path((charge_point_id, connector_id)): Path<(String, u32)>,
) -> Result<Json<ApiResponse<ConnectorDto>>, (StatusCode, Json<ApiResponse<ConnectorDto>>)> {
    match state.repos.charge_points().find_by_id(&charge_point_id).await {
        Ok(Some(cp)) => match cp.connectors.into_iter().find(|c| c.id == connector_id) {
            Some(connector) => Ok(Json(ApiResponse::success(ConnectorDto::from_domain(
                connector,
            )))),
            None => Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!(
                    "Connector {} not found on charge point '{}'",
                    connector_id, charge_point_id
                ))),
            )),
        },
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' not found",
                charge_point_id
            ))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/connectors",
    tag = "Connectors",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    request_body = CreateConnectorRequest,
    responses(
        (status = 201, description = "Connector created", body = ApiResponse<ConnectorDto>),
        (status = 404, description = "Not found"),
        (status = 409, description = "Already exists")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn create_connector(
    State(state): State<AppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<CreateConnectorRequest>,
) -> Result<
    (StatusCode, Json<ApiResponse<ConnectorDto>>),
    (StatusCode, Json<ApiResponse<ConnectorDto>>),
> {
    if request.connector_id < 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Connector ID must be >= 1")),
        ));
    }

    let mut cp = match state.repos.charge_points().find_by_id(&charge_point_id).await {
        Ok(Some(cp)) => cp,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!(
                    "Charge point '{}' not found",
                    charge_point_id
                ))),
            ));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            ));
        }
    };

    if !cp.add_connector(request.connector_id) {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::error(format!(
                "Connector {} already exists on '{}'",
                request.connector_id, charge_point_id
            ))),
        ));
    }

    if let Err(e) = state.repos.charge_points().update(cp.clone()).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ));
    }

    let connector = cp
        .get_connector(request.connector_id)
        .cloned()
        .expect("connector was just added");

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(ConnectorDto::from_domain(connector))),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/charge-points/{charge_point_id}/connectors/{connector_id}",
    tag = "Connectors",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID"),
        ("connector_id" = u32, Path, description = "Connector number")
    ),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Connector in use")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn delete_connector(
    State(state): State<AppState>,
    Path((charge_point_id, connector_id)): Path<(String, u32)>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut cp = match state.repos.charge_points().find_by_id(&charge_point_id).await {
        Ok(Some(cp)) => cp,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!(
                    "Charge point '{}' not found",
                    charge_point_id
                ))),
            ));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            ));
        }
    };

    let connector = match cp.get_connector(connector_id) {
        Some(c) => c.clone(),
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!(
                    "Connector {} not found on '{}'",
                    connector_id, charge_point_id
                ))),
            ));
        }
    };

    if connector.status == crate::domain::ConnectorStatus::Charging {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::error(format!(
                "Cannot delete connector {} — currently charging",
                connector_id
            ))),
        ));
    }

    cp.remove_connector(connector_id);

    if let Err(e) = state.repos.charge_points().update(cp).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ));
    }

    Ok(Json(ApiResponse::success(())))
}

// ── Password management ────────────────────────────────────────

#[utoipa::path(
    put,
    path = "/api/v1/charge-points/{charge_point_id}/password",
    tag = "Charge Points",
    request_body = SetPasswordRequest,
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    responses(
        (status = 200, description = "Password set successfully", body = ApiResponse<SetPasswordResponse>),
        (status = 404, description = "Charge point not found"),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn set_password(
    State(state): State<AppState>,
    Path(charge_point_id): Path<String>,
    ValidatedJson(body): ValidatedJson<SetPasswordRequest>,
) -> Result<
    Json<ApiResponse<SetPasswordResponse>>,
    (StatusCode, Json<ApiResponse<SetPasswordResponse>>),
> {
    // Verify charge point exists
    match state.repos.charge_points().find_by_id(&charge_point_id).await {
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!(
                    "Charge point '{}' not found",
                    charge_point_id
                ))),
            ));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            ));
        }
        Ok(Some(_)) => {}
    }

    // Hash the password
    let password_hash = match hash_password(&body.password) {
        Ok(hash) => hash,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Failed to hash password: {}", e))),
            ));
        }
    };

    // Store the hash
    if let Err(e) = state
        .repos
        .charge_points()
        .set_password_hash(&charge_point_id, Some(password_hash))
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ));
    }

    Ok(Json(ApiResponse::success(SetPasswordResponse {
        message: format!(
            "Password set for charge point '{}'. Use Basic Auth for WS connections.",
            charge_point_id
        ),
        has_password: true,
    })))
}

#[utoipa::path(
    delete,
    path = "/api/v1/charge-points/{charge_point_id}/password",
    tag = "Charge Points",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    responses(
        (status = 200, description = "Password removed", body = ApiResponse<SetPasswordResponse>),
        (status = 404, description = "Charge point not found")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn remove_password(
    State(state): State<AppState>,
    Path(charge_point_id): Path<String>,
) -> Result<
    Json<ApiResponse<SetPasswordResponse>>,
    (StatusCode, Json<ApiResponse<SetPasswordResponse>>),
> {
    match state.repos.charge_points().find_by_id(&charge_point_id).await {
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!(
                    "Charge point '{}' not found",
                    charge_point_id
                ))),
            ));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            ));
        }
        Ok(Some(_)) => {}
    }

    if let Err(e) = state
        .repos
        .charge_points()
        .set_password_hash(&charge_point_id, None)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ));
    }

    Ok(Json(ApiResponse::success(SetPasswordResponse {
        message: format!(
            "Password removed for charge point '{}'. Basic Auth will be rejected.",
            charge_point_id
        ),
        has_password: false,
    })))
}
