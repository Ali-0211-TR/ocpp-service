//! Transaction API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use crate::api::dto::{ApiResponse, PaginatedResponse, PaginationParams, TransactionDto, TransactionFilter};
use crate::infrastructure::Storage;

/// Transaction storage state
#[derive(Clone)]
pub struct TransactionAppState {
    pub storage: Arc<dyn Storage>,
}

/// List transactions for a specific charge point
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
        (status = 200, description = "List of transactions", body = PaginatedResponse<TransactionDto>)
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

/// List all transactions across all charge points
#[utoipa::path(
    get,
    path = "/api/v1/transactions",
    tag = "Transactions",
    params(PaginationParams),
    responses(
        (status = 200, description = "List of all transactions", body = PaginatedResponse<TransactionDto>)
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

/// Get transaction by ID
#[utoipa::path(
    get,
    path = "/api/v1/transactions/{id}",
    tag = "Transactions",
    params(
        ("id" = i32, Path, description = "Transaction ID")
    ),
    responses(
        (status = 200, description = "Transaction details", body = ApiResponse<TransactionDto>),
        (status = 404, description = "Transaction not found")
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

/// Get active transactions for a charge point
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/transactions/active",
    tag = "Transactions",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    responses(
        (status = 200, description = "Active transactions", body = ApiResponse<Vec<TransactionDto>>)
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

/// Transaction statistics
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TransactionStats {
    /// Total number of transactions
    pub total: u32,
    /// Number of active (ongoing) transactions
    pub active: u32,
    /// Number of completed transactions
    pub completed: u32,
    /// Total energy delivered in kWh
    pub total_energy_kwh: f64,
}

/// Get transaction statistics for a charge point
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/transactions/stats",
    tag = "Transactions",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    responses(
        (status = 200, description = "Transaction statistics", body = ApiResponse<TransactionStats>)
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
