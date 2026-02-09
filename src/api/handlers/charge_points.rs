//! Charge Point API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::dto::{ApiResponse, ChargePointDto, ChargePointStats, ConnectorDto, CreateConnectorRequest};
use crate::infrastructure::Storage;
use crate::session::SharedSessionManager;

/// Application state for API handlers
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub session_manager: SharedSessionManager,
}

/// Список всех зарядных станций
///
/// Возвращает полный список станций с информацией
/// о коннекторах и текущем онлайн-статусе.
/// Станции регистрируются автоматически при первом WebSocket-подключении.
#[utoipa::path(
    get,
    path = "/api/v1/charge-points",
    tag = "Charge Points",
    responses(
        (status = 200, description = "Список всех зарядных станций", body = ApiResponse<Vec<ChargePointDto>>)
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

/// Получение станции по ID
///
/// Возвращает полную информацию о станции включая
/// вендора, модель, прошивку, список коннекторов и их статусы.
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}",
    tag = "Charge Points",
    params(
        ("charge_point_id" = String, Path, description = "Уникальный идентификатор станции")
    ),
    responses(
        (status = 200, description = "Полная информация о станции", body = ApiResponse<ChargePointDto>),
        (status = 404, description = "Станция не найдена")
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

/// Удаление зарядной станции
///
/// Полностью удаляет станцию из системы.
/// Не удаляет связанные транзакции — они сохраняются для истории.
#[utoipa::path(
    delete,
    path = "/api/v1/charge-points/{charge_point_id}",
    tag = "Charge Points",
    params(
        ("charge_point_id" = String, Path, description = "ID станции для удаления")
    ),
    responses(
        (status = 200, description = "Станция успешно удалена"),
        (status = 404, description = "Станция не найдена")
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

/// Статистика по станциям
///
/// Возвращает общее количество станций, онлайн, офлайн и заряжающих.
/// Используйте для дашборда.
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/stats",
    tag = "Charge Points",
    responses(
        (status = 200, description = "Статистика: total, online, offline, charging", body = ApiResponse<ChargePointStats>)
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

/// Список онлайн-станций
///
/// Возвращает массив ID станций, которые сейчас подключены
/// по WebSocket. Лёгкий эндпойнт для быстрой проверки.
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/online",
    tag = "Charge Points",
    responses(
        (status = 200, description = "Массив ID онлайн-станций", body = ApiResponse<Vec<String>>)
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

/// Список коннекторов станции
///
/// Возвращает все коннекторы (разъёмы) указанной станции
/// с их текущими статусами и кодами ошибок.
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/connectors",
    tag = "Connectors",
    params(
        ("charge_point_id" = String, Path, description = "ID зарядной станции")
    ),
    responses(
        (status = 200, description = "Список коннекторов станции", body = ApiResponse<Vec<ConnectorDto>>),
        (status = 404, description = "Станция не найдена")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
)]
pub async fn list_connectors(
    State(state): State<AppState>,
    Path(charge_point_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<ConnectorDto>>>, (StatusCode, Json<ApiResponse<Vec<ConnectorDto>>>)>
{
    match state.storage.get_charge_point(&charge_point_id).await {
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

/// Получение коннектора по ID
///
/// Возвращает информацию о конкретном коннекторе (разъёме) станции.
/// connector_id — номер физического разъёма (1-based).
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/connectors/{connector_id}",
    tag = "Connectors",
    params(
        ("charge_point_id" = String, Path, description = "ID зарядной станции"),
        ("connector_id" = u32, Path, description = "Номер коннектора (1-based)")
    ),
    responses(
        (status = 200, description = "Информация о коннекторе", body = ApiResponse<ConnectorDto>),
        (status = 404, description = "Станция или коннектор не найдены")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
)]
pub async fn get_connector(
    State(state): State<AppState>,
    Path((charge_point_id, connector_id)): Path<(String, u32)>,
) -> Result<Json<ApiResponse<ConnectorDto>>, (StatusCode, Json<ApiResponse<ConnectorDto>>)> {
    match state.storage.get_charge_point(&charge_point_id).await {
        Ok(Some(cp)) => {
            match cp.connectors.into_iter().find(|c| c.id == connector_id) {
                Some(connector) => {
                    Ok(Json(ApiResponse::success(ConnectorDto::from_domain(connector))))
                }
                None => Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error(format!(
                        "Connector {} not found on charge point '{}'",
                        connector_id, charge_point_id
                    ))),
                )),
            }
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

/// Добавление коннектора к станции
///
/// Создаёт новый коннектор с указанным ID на зарядной станции.
/// Начальный статус — Unavailable.
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/connectors",
    tag = "Connectors",
    params(
        ("charge_point_id" = String, Path, description = "ID зарядной станции")
    ),
    request_body = CreateConnectorRequest,
    responses(
        (status = 201, description = "Коннектор создан", body = ApiResponse<ConnectorDto>),
        (status = 404, description = "Станция не найдена"),
        (status = 409, description = "Коннектор с таким ID уже существует")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
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
            Json(ApiResponse::error(
                "Connector ID must be >= 1. Connector 0 is reserved for the charge point itself (OCPP 1.6)".to_string(),
            )),
        ));
    }

    let mut cp = match state.storage.get_charge_point(&charge_point_id).await {
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
                "Connector {} already exists on charge point '{}'",
                request.connector_id, charge_point_id
            ))),
        ));
    }

    if let Err(e) = state.storage.update_charge_point(cp.clone()).await {
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

/// Удаление коннектора со станции
///
/// Удаляет коннектор по его ID. Нельзя удалить коннектор, на котором идёт активная зарядка.
#[utoipa::path(
    delete,
    path = "/api/v1/charge-points/{charge_point_id}/connectors/{connector_id}",
    tag = "Connectors",
    params(
        ("charge_point_id" = String, Path, description = "ID зарядной станции"),
        ("connector_id" = u32, Path, description = "Номер коннектора")
    ),
    responses(
        (status = 200, description = "Коннектор удалён"),
        (status = 404, description = "Станция или коннектор не найдены"),
        (status = 409, description = "Коннектор используется (идёт зарядка)")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    ),
)]
pub async fn delete_connector(
    State(state): State<AppState>,
    Path((charge_point_id, connector_id)): Path<(String, u32)>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut cp = match state.storage.get_charge_point(&charge_point_id).await {
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
                    "Connector {} not found on charge point '{}'",
                    connector_id, charge_point_id
                ))),
            ));
        }
    };

    if connector.status == crate::domain::ConnectorStatus::Charging {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::error(format!(
                "Cannot delete connector {} — it is currently charging",
                connector_id
            ))),
        ));
    }

    cp.remove_connector(connector_id);

    if let Err(e) = state.storage.update_charge_point(cp).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ));
    }

    Ok(Json(ApiResponse::success(())))
}