//! Transaction API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use log::info;

use crate::api::dto::{ApiResponse, PaginatedResponse, PaginationParams, TransactionDto, TransactionFilter};
use crate::infrastructure::Storage;

/// Transaction storage state
#[derive(Clone)]
pub struct TransactionAppState {
    pub storage: Arc<dyn Storage>,
}

/// Транзакции конкретной станции
///
/// Возвращает список транзакций с фильтрацией по статусу и датам.
/// Поддерживает пагинацию.
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/transactions",
    tag = "Transactions",
    params(
        ("charge_point_id" = String, Path, description = "ID зарядной станции"),
        TransactionFilter,
        PaginationParams
    ),
    responses(
        (status = 200, description = "Список транзакций с пагинацией", body = PaginatedResponse<TransactionDto>)
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn list_transactions_for_charge_point(
    State(state): State<TransactionAppState>,
    Path(charge_point_id): Path<String>,
    Query(filter): Query<TransactionFilter>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<TransactionDto>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state
        .storage
        .list_transactions_for_charge_point(&charge_point_id)
        .await
    {
        Ok(transactions) => {
            // Apply filters
            let filtered: Vec<_> = transactions
                .into_iter()
                .filter(|t| {
                    // Filter by status
                    if let Some(ref status) = filter.status {
                        let tx_status = match t.status {
                            crate::domain::TransactionStatus::Active => "active",
                            crate::domain::TransactionStatus::Completed => "completed",
                            crate::domain::TransactionStatus::Failed => "failed",
                        };
                        if !status.eq_ignore_ascii_case(tx_status) {
                            return false;
                        }
                    }

                    // Filter by start date
                    if let Some(ref from) = filter.from_date {
                        if t.started_at < *from {
                            return false;
                        }
                    }

                    // Filter by end date
                    if let Some(ref to) = filter.to_date {
                        if t.started_at > *to {
                            return false;
                        }
                    }

                    true
                })
                .collect();

            // Pagination
            let total = filtered.len() as u64;
            let page = pagination.page;
            let limit = pagination.limit;

            let start = ((page - 1) * limit) as usize;
            let items: Vec<TransactionDto> = filtered
                .into_iter()
                .skip(start)
                .take(limit as usize)
                .map(TransactionDto::from_domain)
                .collect();

            Ok(Json(PaginatedResponse::new(items, total, page, limit)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Все транзакции по всем станциям
///
/// Возвращает все транзакции с пагинацией.
/// Для общего обзора истории зарядок.
#[utoipa::path(
    get,
    path = "/api/v1/transactions",
    tag = "Transactions",
    params(PaginationParams),
    responses(
        (status = 200, description = "Список всех транзакций с пагинацией", body = PaginatedResponse<TransactionDto>)
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn list_all_transactions(
    State(state): State<TransactionAppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<TransactionDto>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.storage.list_all_transactions().await {
        Ok(transactions) => {
            let total = transactions.len() as u64;
            let page = pagination.page;
            let limit = pagination.limit;
            let start = ((page - 1) * limit) as usize;
            let items: Vec<TransactionDto> = transactions
                .into_iter()
                .skip(start)
                .take(limit as usize)
                .map(TransactionDto::from_domain)
                .collect();
            Ok(Json(PaginatedResponse::new(items, total, page, limit)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Получение транзакции по ID
///
/// Возвращает полную информацию о транзакции:
/// показания счётчика, потреблённая энергия, время, статус.
#[utoipa::path(
    get,
    path = "/api/v1/transactions/{id}",
    tag = "Transactions",
    params(
        ("id" = i32, Path, description = "ID транзакции")
    ),
    responses(
        (status = 200, description = "Полная информация о транзакции", body = ApiResponse<TransactionDto>),
        (status = 404, description = "Транзакция не найдена")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_transaction(
    State(state): State<TransactionAppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<TransactionDto>>, (StatusCode, Json<ApiResponse<TransactionDto>>)> {
    match state.storage.get_transaction(id).await {
        Ok(Some(tx)) => Ok(Json(ApiResponse::success(TransactionDto::from_domain(tx)))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("Transaction {} not found", id))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Активные транзакции станции
///
/// Возвращает только текущие (незавершённые) зарядные сессии.
/// Используйте для получения `transaction_id` перед RemoteStop.
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/transactions/active",
    tag = "Transactions",
    params(
        ("charge_point_id" = String, Path, description = "ID зарядной станции")
    ),
    responses(
        (status = 200, description = "Список активных транзакций", body = ApiResponse<Vec<TransactionDto>>)
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_active_transactions(
    State(state): State<TransactionAppState>,
    Path(charge_point_id): Path<String>,
) -> Result<
    Json<ApiResponse<Vec<TransactionDto>>>,
    (StatusCode, Json<ApiResponse<Vec<TransactionDto>>>),
> {
    match state
        .storage
        .list_transactions_for_charge_point(&charge_point_id)
        .await
    {
        Ok(transactions) => {
            let active: Vec<TransactionDto> = transactions
                .into_iter()
                .filter(|t| t.status == crate::domain::TransactionStatus::Active)
                .map(TransactionDto::from_domain)
                .collect();

            Ok(Json(ApiResponse::success(active)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Статистика транзакций станции
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TransactionStats {
    /// Общее количество транзакций для этой станции
    pub total: u32,
    /// Количество активных (текущих) зарядок
    pub active: u32,
    /// Количество завершённых зарядок
    pub completed: u32,
    /// Общая отданная энергия в кВт·ч (округлено до 2 знаков)
    pub total_energy_kwh: f64,
}

/// Статистика транзакций станции
///
/// Возвращает: общее количество, активные, завершённые
/// и общую отданную энергию (кВт·ч).
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/transactions/stats",
    tag = "Transactions",
    params(
        ("charge_point_id" = String, Path, description = "ID зарядной станции")
    ),
    responses(
        (status = 200, description = "Статистика: total, active, completed, energy", body = ApiResponse<TransactionStats>)
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_transaction_stats(
    State(state): State<TransactionAppState>,
    Path(charge_point_id): Path<String>,
) -> Result<Json<ApiResponse<TransactionStats>>, (StatusCode, Json<ApiResponse<TransactionStats>>)>
{
    match state
        .storage
        .list_transactions_for_charge_point(&charge_point_id)
        .await
    {
        Ok(transactions) => {
            let total = transactions.len() as u32;
            let mut active = 0u32;
            let mut completed = 0u32;
            let mut total_energy = 0.0f64;

            for tx in &transactions {
                match tx.status {
                    crate::domain::TransactionStatus::Active => active += 1,
                    crate::domain::TransactionStatus::Completed => {
                        completed += 1;
                        // Calculate energy delivered
                        if let Some(energy) = tx.energy_consumed() {
                            total_energy += energy as f64 / 1000.0; // Wh to kWh
                        }
                    }
                    crate::domain::TransactionStatus::Failed => {}
                }
            }

            let stats = TransactionStats {
                total,
                active,
                completed,
                total_energy_kwh: (total_energy * 100.0).round() / 100.0,
            };

            Ok(Json(ApiResponse::success(stats)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Принудительная остановка зависшей транзакции
///
/// Используется когда транзакция зависла в статусе `Active` из-за
/// ошибки связи или некорректного поведения станции.
/// Устанавливает `meter_stop = meter_start` (0 Wh потреблено),
/// статус `Completed` и причину `ForceStop`.
///
/// ⚠️ Не отправляет команду на станцию — только обновляет базу данных.
/// Если станция всё ещё заряжает, используйте `remote-stop` вместо этого.
#[utoipa::path(
    post,
    path = "/api/v1/transactions/{transaction_id}/force-stop",
    tag = "Transactions",
    params(
        ("transaction_id" = i32, Path, description = "ID зависшей транзакции")
    ),
    responses(
        (status = 200, description = "Транзакция принудительно остановлена", body = ApiResponse<TransactionDto>),
        (status = 404, description = "Транзакция не найдена"),
        (status = 409, description = "Транзакция уже завершена")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn force_stop_transaction(
    State(state): State<TransactionAppState>,
    Path(transaction_id): Path<i32>,
) -> Result<Json<ApiResponse<TransactionDto>>, (StatusCode, Json<ApiResponse<TransactionDto>>)> {
    // Get the transaction
    let transaction = match state.storage.get_transaction(transaction_id).await {
        Ok(Some(tx)) => tx,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!(
                    "Транзакция {} не найдена",
                    transaction_id
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

    // Check if already stopped
    if !transaction.is_active() {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::error(format!(
                "Транзакция {} уже завершена (статус: {:?})",
                transaction_id, transaction.status
            ))),
        ));
    }

    // Force stop: set meter_stop to meter_start (0 energy consumed)
    let mut tx = transaction;
    tx.stop(tx.meter_start, Some("ForceStop".to_string()));

    if let Err(e) = state.storage.update_transaction(tx.clone()).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ));
    }

    info!(
        "Transaction {} force-stopped (CP: {}, Connector: {})",
        transaction_id, tx.charge_point_id, tx.connector_id
    );

    Ok(Json(ApiResponse::success(TransactionDto::from_domain(tx))))
}
