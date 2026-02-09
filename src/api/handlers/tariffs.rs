//! Tariff REST API handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::ApiResponse;
use crate::api::handlers::AppState;
use crate::domain::{Tariff, TariffType};

/// Тариф на зарядку
///
/// Определяет стоимость зарядной сессии.
/// Все цены указаны в минимальных единицах валюты (копейки, центы).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TariffResponse {
    /// Уникальный ID тарифа
    pub id: i32,
    /// Название тарифа
    pub name: String,
    /// Описание
    pub description: Option<String>,
    /// Тип тарифа: `PerKwh`, `PerMinute`, `PerSession`, `Combined`
    pub tariff_type: String,
    /// Цена за кВт·ч (в мин. единицах, напр. копейки)
    pub price_per_kwh: i32,
    /// Цена за минуту (в мин. единицах)
    pub price_per_minute: i32,
    /// Стартовый сбор за сессию (в мин. единицах)
    pub session_fee: i32,
    /// Код валюты (ISO 4217: `UZS`, `USD`, `EUR`)
    pub currency: String,
    /// Минимальная стоимость сессии
    pub min_fee: i32,
    /// Максимальная стоимость (0 = без ограничения)
    pub max_fee: i32,
    /// Активен ли тариф
    pub is_active: bool,
    /// Тариф по умолчанию (применяется если не указан конкретный)
    pub is_default: bool,
    /// Начало действия (ISO 8601)
    pub valid_from: Option<DateTime<Utc>>,
    /// Конец действия (ISO 8601)
    pub valid_until: Option<DateTime<Utc>>,
    /// Дата создания
    pub created_at: DateTime<Utc>,
    /// Дата последнего обновления
    pub updated_at: DateTime<Utc>,
}

impl From<Tariff> for TariffResponse {
    fn from(t: Tariff) -> Self {
        Self {
            id: t.id,
            name: t.name,
            description: t.description,
            tariff_type: t.tariff_type.to_string(),
            price_per_kwh: t.price_per_kwh,
            price_per_minute: t.price_per_minute,
            session_fee: t.session_fee,
            currency: t.currency,
            min_fee: t.min_fee,
            max_fee: t.max_fee,
            is_active: t.is_active,
            is_default: t.is_default,
            valid_from: t.valid_from,
            valid_until: t.valid_until,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

/// Запрос на создание тарифа
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTariffRequest {
    /// Название тарифа
    pub name: String,
    /// Описание
    pub description: Option<String>,
    /// Тип тарифа: `PerKwh`, `PerMinute`, `PerSession`, `Combined`
    pub tariff_type: String,
    /// Цена за кВт·ч в минимальных единицах валюты (напр. копейки)
    pub price_per_kwh: i32,
    /// Цена за минуту в минимальных единицах
    pub price_per_minute: i32,
    /// Стартовый сбор за сессию в минимальных единицах
    pub session_fee: i32,
    /// Код валюты (ISO 4217: `UZS`, `USD`, `EUR`)
    pub currency: String,
    /// Минимальная стоимость сессии (по умолчанию 0)
    pub min_fee: Option<i32>,
    /// Максимальная стоимость (0 = без ограничения, по умолчанию 0)
    pub max_fee: Option<i32>,
    /// Активен ли тариф (по умолчанию true)
    pub is_active: Option<bool>,
    /// Тариф по умолчанию (по умолчанию false)
    pub is_default: Option<bool>,
    /// Начало действия (ISO 8601)
    pub valid_from: Option<DateTime<Utc>>,
    /// Конец действия (ISO 8601)
    pub valid_until: Option<DateTime<Utc>>,
}

/// Запрос на обновление тарифа (partial update)
///
/// Передайте только изменяемые поля.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTariffRequest {
    /// Новое название
    pub name: Option<String>,
    /// Новое описание
    pub description: Option<String>,
    /// Новый тип тарифа
    pub tariff_type: Option<String>,
    /// Новая цена за кВт·ч
    pub price_per_kwh: Option<i32>,
    /// Новая цена за минуту
    pub price_per_minute: Option<i32>,
    /// Новый стартовый сбор
    pub session_fee: Option<i32>,
    /// Новый код валюты
    pub currency: Option<String>,
    /// Новая минимальная стоимость
    pub min_fee: Option<i32>,
    /// Новая максимальная стоимость
    pub max_fee: Option<i32>,
    /// Активность тарифа
    pub is_active: Option<bool>,
    /// Тариф по умолчанию
    pub is_default: Option<bool>,
    /// Начало действия
    pub valid_from: Option<DateTime<Utc>>,
    /// Конец действия
    pub valid_until: Option<DateTime<Utc>>,
}

/// Запрос на предварительный расчёт стоимости
///
/// Используйте для показа оценки стоимости до зарядки.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CostPreviewRequest {
    /// ID тарифа. Если не указан — используется тариф по умолчанию
    pub tariff_id: Option<i32>,
    /// Потреблённая энергия в Вт·ч (напр. 5000 = 5 кВт·ч)
    pub energy_wh: i32,
    /// Продолжительность сессии в секундах (напр. 3600 = 1 час)
    pub duration_seconds: i64,
}

/// Детализация стоимости зарядки
///
/// Все суммы в минимальных единицах валюты.
#[derive(Debug, Serialize, ToSchema)]
pub struct CostBreakdownResponse {
    /// Стоимость энергии (energy_wh × price_per_kwh / 1000)
    pub energy_cost: i32,
    /// Стоимость времени (duration_minutes × price_per_minute)
    pub time_cost: i32,
    /// Стартовый сбор
    pub session_fee: i32,
    /// Промежуточный итог до применения min/max
    pub subtotal: i32,
    /// Итоговая сумма после применения min_fee / max_fee
    pub total: i32,
    /// Код валюты (ISO 4217)
    pub currency: String,
    /// Форматированная строка итога (напр. "12.50 UZS")
    pub formatted_total: String,
}

fn parse_tariff_type(s: &str) -> TariffType {
    match s {
        "PerMinute" => TariffType::PerMinute,
        "PerSession" => TariffType::PerSession,
        "Combined" => TariffType::Combined,
        _ => TariffType::PerKwh,
    }
}

/// Список всех тарифов
#[utoipa::path(
    get,
    path = "/api/v1/tariffs",
    tag = "Tariffs",
    responses(
        (status = 200, description = "Список тарифов", body = ApiResponse<Vec<TariffResponse>>)
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn list_tariffs(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<TariffResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.storage.list_tariffs().await {
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

/// Получение тарифа по ID
#[utoipa::path(
    get,
    path = "/api/v1/tariffs/{id}",
    tag = "Tariffs",
    params(
        ("id" = i32, Path, description = "ID тарифа")
    ),
    responses(
        (status = 200, description = "Полная информация о тарифе", body = ApiResponse<TariffResponse>),
        (status = 404, description = "Тариф не найден")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn get_tariff(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<TariffResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.storage.get_tariff(id).await {
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

/// Тариф по умолчанию
///
/// Применяется когда не указан конкретный тариф.
#[utoipa::path(
    get,
    path = "/api/v1/tariffs/default",
    tag = "Tariffs",
    responses(
        (status = 200, description = "Тариф по умолчанию", body = ApiResponse<TariffResponse>),
        (status = 404, description = "Тариф по умолчанию не настроен")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn get_default_tariff(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<TariffResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.storage.get_default_tariff().await {
        Ok(Some(tariff)) => Ok(Json(ApiResponse::success(tariff.into()))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("No default tariff configured")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Failed to get default tariff: {}", e))),
        )),
    }
}

/// Создание нового тарифа
#[utoipa::path(
    post,
    path = "/api/v1/tariffs",
    tag = "Tariffs",
    request_body = CreateTariffRequest,
    responses(
        (status = 201, description = "Тариф успешно создан", body = ApiResponse<TariffResponse>),
        (status = 400, description = "Некорректные данные")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn create_tariff(
    State(state): State<AppState>,
    Json(req): Json<CreateTariffRequest>,
) -> Result<(StatusCode, Json<ApiResponse<TariffResponse>>), (StatusCode, Json<ApiResponse<()>>)> {
    let now = Utc::now();
    
    let tariff = Tariff {
        id: 0, // Will be set by database
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

    match state.storage.save_tariff(tariff).await {
        Ok(saved) => Ok((StatusCode::CREATED, Json(ApiResponse::success(saved.into())))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Failed to create tariff: {}", e))),
        )),
    }
}

/// Обновление тарифа
///
/// Partial update — передайте только изменяемые поля.
#[utoipa::path(
    put,
    path = "/api/v1/tariffs/{id}",
    tag = "Tariffs",
    params(
        ("id" = i32, Path, description = "ID тарифа")
    ),
    request_body = UpdateTariffRequest,
    responses(
        (status = 200, description = "Тариф успешно обновлён", body = ApiResponse<TariffResponse>),
        (status = 404, description = "Тариф не найден")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn update_tariff(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTariffRequest>,
) -> Result<Json<ApiResponse<TariffResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Get existing tariff
    let existing = match state.storage.get_tariff(id).await {
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
        tariff_type: req.tariff_type.map(|t| parse_tariff_type(&t)).unwrap_or(existing.tariff_type),
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

    match state.storage.update_tariff(updated.clone()).await {
        Ok(()) => Ok(Json(ApiResponse::success(updated.into()))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Failed to update tariff: {}", e))),
        )),
    }
}

/// Удаление тарифа
#[utoipa::path(
    delete,
    path = "/api/v1/tariffs/{id}",
    tag = "Tariffs",
    params(
        ("id" = i32, Path, description = "ID тарифа")
    ),
    responses(
        (status = 200, description = "Тариф успешно удалён"),
        (status = 404, description = "Тариф не найден")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn delete_tariff(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.storage.delete_tariff(id).await {
        Ok(()) => Ok(Json(ApiResponse::success("Tariff deleted".to_string()))),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("Failed to delete tariff: {}", e))),
        )),
    }
}

/// Предварительный расчёт стоимости зарядки
///
/// Показывает детализированную разбивку стоимости:
/// энергия + время + стартовый сбор = итого.
#[utoipa::path(
    post,
    path = "/api/v1/tariffs/preview-cost",
    tag = "Tariffs",
    request_body = CostPreviewRequest,
    responses(
        (status = 200, description = "Детализация стоимости", body = ApiResponse<CostBreakdownResponse>),
        (status = 404, description = "Тариф не найден")
    ),
    security(("bearer_auth" = []), ("api_key" = []))
)]
pub async fn preview_cost(
    State(state): State<AppState>,
    Json(req): Json<CostPreviewRequest>,
) -> Result<Json<ApiResponse<CostBreakdownResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Get tariff
    let tariff = if let Some(id) = req.tariff_id {
        match state.storage.get_tariff(id).await {
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
        match state.storage.get_default_tariff().await {
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
