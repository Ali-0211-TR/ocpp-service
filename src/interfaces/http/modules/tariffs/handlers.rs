//! Tariff REST API handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;

use super::dto::{
    CostBreakdownResponse, CostPreviewRequest, CreateTariffRequest, TariffResponse,
    UpdateTariffRequest,
};
use crate::domain::{Tariff, TariffType};
use crate::interfaces::http::modules::charge_points::AppState;
use crate::interfaces::http::common::ApiResponse;

fn parse_tariff_type(s: &str) -> TariffType {
    match s {
        "PerMinute" => TariffType::PerMinute,
        "PerSession" => TariffType::PerSession,
        "Combined" => TariffType::Combined,
        _ => TariffType::PerKwh,
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/tariffs",
    tag = "Tariffs",
    security(("bearer_auth" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "Tariff list", body = ApiResponse<Vec<TariffResponse>>)
    )
)]
pub async fn list_tariffs(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<TariffResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.repos.tariffs().find_all().await {
        Ok(tariffs) => {
            let responses: Vec<TariffResponse> = tariffs.into_iter().map(Into::into).collect();
            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Failed to list tariffs: {}", e))),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/tariffs/{id}",
    tag = "Tariffs",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("id" = i32, Path, description = "Tariff ID")),
    responses(
        (status = 200, description = "Tariff details", body = ApiResponse<TariffResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_tariff(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<TariffResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.repos.tariffs().find_by_id(id).await {
        Ok(Some(tariff)) => Ok(Json(ApiResponse::success(tariff.into()))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("Tariff {} not found", id))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Failed to get tariff: {}", e))),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/tariffs/default",
    tag = "Tariffs",
    security(("bearer_auth" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "Default tariff", body = ApiResponse<TariffResponse>),
        (status = 404, description = "No default tariff configured")
    )
)]
pub async fn get_default_tariff(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<TariffResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.repos.tariffs().find_default().await {
        Ok(Some(tariff)) => Ok(Json(ApiResponse::success(tariff.into()))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("No default tariff configured")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!(
                "Failed to get default tariff: {}",
                e
            ))),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/tariffs",
    tag = "Tariffs",
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = CreateTariffRequest,
    responses(
        (status = 201, description = "Created", body = ApiResponse<TariffResponse>),
        (status = 400, description = "Invalid data")
    )
)]
pub async fn create_tariff(
    State(state): State<AppState>,
    Json(req): Json<CreateTariffRequest>,
) -> Result<(StatusCode, Json<ApiResponse<TariffResponse>>), (StatusCode, Json<ApiResponse<()>>)> {
    let now = Utc::now();

    let tariff = Tariff {
        id: 0,
        name: req.name,
        description: req.description,
        tariff_type: parse_tariff_type(&req.tariff_type),
        price_per_kwh: req.price_per_kwh,
        price_per_minute: req.price_per_minute,
        session_fee: req.session_fee,
        currency: req.currency,
        min_fee: req.min_fee.unwrap_or(0),
        max_fee: req.max_fee.unwrap_or(0),
        is_active: req.is_active.unwrap_or(true),
        is_default: req.is_default.unwrap_or(false),
        valid_from: req.valid_from,
        valid_until: req.valid_until,
        created_at: now,
        updated_at: now,
    };

    match state.repos.tariffs().save(tariff).await {
        Ok(saved) => Ok((
            StatusCode::CREATED,
            Json(ApiResponse::success(saved.into())),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Failed to create tariff: {}",
                e
            ))),
        )),
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/tariffs/{id}",
    tag = "Tariffs",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("id" = i32, Path, description = "Tariff ID")),
    request_body = UpdateTariffRequest,
    responses(
        (status = 200, description = "Updated", body = ApiResponse<TariffResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_tariff(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTariffRequest>,
) -> Result<Json<ApiResponse<TariffResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let existing = match state.repos.tariffs().find_by_id(id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!("Tariff {} not found", id))),
            ));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Failed to get tariff: {}", e))),
            ));
        }
    };

    let updated = Tariff {
        id: existing.id,
        name: req.name.unwrap_or(existing.name),
        description: req.description.or(existing.description),
        tariff_type: req
            .tariff_type
            .map(|t| parse_tariff_type(&t))
            .unwrap_or(existing.tariff_type),
        price_per_kwh: req.price_per_kwh.unwrap_or(existing.price_per_kwh),
        price_per_minute: req.price_per_minute.unwrap_or(existing.price_per_minute),
        session_fee: req.session_fee.unwrap_or(existing.session_fee),
        currency: req.currency.unwrap_or(existing.currency),
        min_fee: req.min_fee.unwrap_or(existing.min_fee),
        max_fee: req.max_fee.unwrap_or(existing.max_fee),
        is_active: req.is_active.unwrap_or(existing.is_active),
        is_default: req.is_default.unwrap_or(existing.is_default),
        valid_from: req.valid_from.or(existing.valid_from),
        valid_until: req.valid_until.or(existing.valid_until),
        created_at: existing.created_at,
        updated_at: Utc::now(),
    };

    match state.repos.tariffs().update(updated.clone()).await {
        Ok(()) => Ok(Json(ApiResponse::success(updated.into()))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!(
                "Failed to update tariff: {}",
                e
            ))),
        )),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/tariffs/{id}",
    tag = "Tariffs",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("id" = i32, Path, description = "Tariff ID")),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_tariff(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.repos.tariffs().delete(id).await {
        Ok(()) => Ok(Json(ApiResponse::success("Tariff deleted".to_string()))),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Failed to delete tariff: {}",
                e
            ))),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/tariffs/preview-cost",
    tag = "Tariffs",
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = CostPreviewRequest,
    responses(
        (status = 200, description = "Cost breakdown", body = ApiResponse<CostBreakdownResponse>),
        (status = 404, description = "Tariff not found")
    )
)]
pub async fn preview_cost(
    State(state): State<AppState>,
    Json(req): Json<CostPreviewRequest>,
) -> Result<Json<ApiResponse<CostBreakdownResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tariff = if let Some(id) = req.tariff_id {
        match state.repos.tariffs().find_by_id(id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error(format!("Tariff {} not found", id))),
                ));
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(format!("Failed to get tariff: {}", e))),
                ));
            }
        }
    } else {
        match state.repos.tariffs().find_default().await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error("No default tariff configured")),
                ));
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(format!("Failed to get tariff: {}", e))),
                ));
            }
        }
    };

    let breakdown = tariff.calculate_cost_breakdown(req.energy_wh, req.duration_seconds);

    Ok(Json(ApiResponse::success(CostBreakdownResponse {
        energy_cost: breakdown.energy_cost,
        time_cost: breakdown.time_cost,
        session_fee: breakdown.session_fee,
        subtotal: breakdown.subtotal,
        total: breakdown.total,
        currency: breakdown.currency.clone(),
        formatted_total: breakdown.format_total(),
    })))
}
