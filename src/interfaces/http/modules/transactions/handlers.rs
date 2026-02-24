//! Transaction API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use tracing::{info, warn};

use super::dto::{TransactionDto, TransactionFilter};
use crate::application::events::{
    ConnectorStatusChangedEvent, Event, SharedEventBus, TransactionBilledEvent,
    TransactionStoppedEvent,
};
use crate::application::BillingService;
use crate::domain::charge_point::ConnectorStatus;
use crate::domain::RepositoryProvider;
use crate::interfaces::http::common::{ApiResponse, PaginatedResponse, PaginationParams};

/// Transaction handler state
#[derive(Clone)]
pub struct TransactionAppState {
    pub repos: Arc<dyn RepositoryProvider>,
    pub billing_service: Arc<BillingService>,
    pub event_bus: SharedEventBus,
}

#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/transactions",
    tag = "Transactions",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID"),
        TransactionFilter,
        PaginationParams
    ),
    responses(
        (status = 200, description = "Transaction list", body = PaginatedResponse<TransactionDto>)
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn list_transactions_for_charge_point(
    State(state): State<TransactionAppState>,
    Path(charge_point_id): Path<String>,
    Query(filter): Query<TransactionFilter>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<TransactionDto>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state
        .repos
        .transactions()
        .find_by_charge_point(&charge_point_id)
        .await
    {
        Ok(transactions) => {
            let filtered: Vec<_> = transactions
                .into_iter()
                .filter(|t| {
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
                    if let Some(ref from) = filter.from_date {
                        if t.started_at < *from {
                            return false;
                        }
                    }
                    if let Some(ref to) = filter.to_date {
                        if t.started_at > *to {
                            return false;
                        }
                    }
                    if let Some(ref eid) = filter.external_order_id {
                        if t.external_order_id.as_deref() != Some(eid.as_str()) {
                            return false;
                        }
                    }
                    true
                })
                .collect();

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

#[utoipa::path(
    get,
    path = "/api/v1/transactions",
    tag = "Transactions",
    params(PaginationParams),
    responses(
        (status = 200, description = "All transactions", body = PaginatedResponse<TransactionDto>)
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn list_all_transactions(
    State(state): State<TransactionAppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<TransactionDto>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.repos.transactions().find_all().await {
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

#[utoipa::path(
    get,
    path = "/api/v1/transactions/{id}",
    tag = "Transactions",
    params(("id" = i32, Path, description = "Transaction ID")),
    responses(
        (status = 200, description = "Transaction details", body = ApiResponse<TransactionDto>),
        (status = 404, description = "Not found")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn get_transaction(
    State(state): State<TransactionAppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<TransactionDto>>, (StatusCode, Json<ApiResponse<TransactionDto>>)> {
    match state.repos.transactions().find_by_id(id).await {
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

#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/transactions/active",
    tag = "Transactions",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    responses(
        (status = 200, description = "Active transactions", body = ApiResponse<Vec<TransactionDto>>)
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn get_active_transactions(
    State(state): State<TransactionAppState>,
    Path(charge_point_id): Path<String>,
) -> Result<
    Json<ApiResponse<Vec<TransactionDto>>>,
    (StatusCode, Json<ApiResponse<Vec<TransactionDto>>>),
> {
    match state
        .repos
        .transactions()
        .find_by_charge_point(&charge_point_id)
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

/// Transaction statistics
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TransactionStats {
    pub total: u32,
    pub active: u32,
    pub completed: u32,
    pub total_energy_kwh: f64,
}

#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/transactions/stats",
    tag = "Transactions",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    responses(
        (status = 200, description = "Transaction stats", body = ApiResponse<TransactionStats>)
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn get_transaction_stats(
    State(state): State<TransactionAppState>,
    Path(charge_point_id): Path<String>,
) -> Result<Json<ApiResponse<TransactionStats>>, (StatusCode, Json<ApiResponse<TransactionStats>>)>
{
    match state
        .repos
        .transactions()
        .find_by_charge_point(&charge_point_id)
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
                        if let Some(energy) = tx.energy_consumed() {
                            total_energy += energy as f64 / 1000.0;
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

#[utoipa::path(
    post,
    path = "/api/v1/transactions/{transaction_id}/force-stop",
    tag = "Transactions",
    params(("transaction_id" = i32, Path, description = "Transaction ID")),
    responses(
        (status = 200, description = "Force-stopped", body = ApiResponse<TransactionDto>),
        (status = 404, description = "Not found"),
        (status = 409, description = "Already stopped")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn force_stop_transaction(
    State(state): State<TransactionAppState>,
    Path(transaction_id): Path<i32>,
) -> Result<Json<ApiResponse<TransactionDto>>, (StatusCode, Json<ApiResponse<TransactionDto>>)> {
    let transaction = match state.repos.transactions().find_by_id(transaction_id).await {
        Ok(Some(tx)) => tx,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!(
                    "Transaction {} not found",
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

    if !transaction.is_active() {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::error(format!(
                "Transaction {} already stopped ({:?})",
                transaction_id, transaction.status
            ))),
        ));
    }

    let mut tx = transaction;
    tx.stop(tx.meter_start, Some("ForceStop".to_string()));

    if let Err(e) = state.repos.transactions().update(tx.clone()).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ));
    }

    info!(
        "Transaction {} force-stopped (CP: {}, Connector: {})",
        transaction_id, tx.charge_point_id, tx.connector_id
    );

    let energy_wh = tx.energy_consumed().unwrap_or(0);

    // Calculate billing for the force-stopped transaction
    let (total_cost, currency) = match state
        .billing_service
        .calculate_transaction_billing(transaction_id, None)
        .await
    {
        Ok(billing) => {
            info!(
                "Billing calculated for force-stopped transaction {}: {} {}",
                transaction_id,
                billing.total_cost as f64 / 100.0,
                billing.currency
            );

            state.event_bus.publish(Event::TransactionBilled(
                TransactionBilledEvent {
                    charge_point_id: tx.charge_point_id.clone(),
                    transaction_id,
                    energy_kwh: energy_wh as f64 / 1000.0,
                    duration_minutes: billing.duration_seconds as f64 / 60.0,
                    energy_cost: billing.energy_cost as f64 / 100.0,
                    time_cost: billing.time_cost as f64 / 100.0,
                    session_fee: billing.session_fee as f64 / 100.0,
                    total_cost: billing.total_cost as f64 / 100.0,
                    currency: billing.currency.clone(),
                    tariff_name: None,
                    timestamp: Utc::now(),
                },
            ));

            (billing.total_cost as f64 / 100.0, billing.currency)
        }
        Err(e) => {
            warn!(
                "Billing failed for force-stopped transaction {}: {}",
                transaction_id, e
            );
            (0.0, "UZS".to_string())
        }
    };

    state.event_bus.publish(Event::TransactionStopped(
        TransactionStoppedEvent {
            charge_point_id: tx.charge_point_id.clone(),
            transaction_id,
            id_tag: Some(tx.id_tag.clone()),
            meter_stop: tx.meter_start,
            energy_consumed_kwh: energy_wh as f64 / 1000.0,
            total_cost,
            currency,
            reason: Some("ForceStop".to_string()),
            timestamp: Utc::now(),
            external_order_id: tx.external_order_id.clone(),
        },
    ));

    // Update connector status to Available since the transaction is force-stopped
    // (the charger is likely disconnected and won't send StatusNotification itself)
    if let Ok(Some(mut cp)) = state
        .repos
        .charge_points()
        .find_by_id(&tx.charge_point_id)
        .await
    {
        cp.update_connector_status(tx.connector_id as u32, ConnectorStatus::Available);
        if let Err(e) = state.repos.charge_points().update(cp).await {
            warn!(
                "Failed to update connector {} status after force-stop: {}",
                tx.connector_id, e
            );
        } else {
            state.event_bus.publish(Event::ConnectorStatusChanged(
                ConnectorStatusChangedEvent {
                    charge_point_id: tx.charge_point_id.clone(),
                    connector_id: tx.connector_id as u32,
                    status: "Available".to_string(),
                    error_code: None,
                    info: Some("Force-stopped transaction".to_string()),
                    timestamp: Utc::now(),
                },
            ));
        }
    }

    Ok(Json(ApiResponse::success(TransactionDto::from_domain(tx))))
}
