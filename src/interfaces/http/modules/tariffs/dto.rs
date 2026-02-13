//! Tariff DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::Tariff;

/// Тариф на зарядку
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TariffResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub tariff_type: String,
    pub price_per_kwh: i32,
    pub price_per_minute: i32,
    pub session_fee: i32,
    pub currency: String,
    pub min_fee: i32,
    pub max_fee: i32,
    pub is_active: bool,
    pub is_default: bool,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
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

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateTariffRequest {
    #[validate(length(min = 1, max = 100, message = "tariff name is required"))]
    pub name: String,
    pub description: Option<String>,
    pub tariff_type: String,
    pub price_per_kwh: i32,
    pub price_per_minute: i32,
    pub session_fee: i32,
    pub currency: String,
    pub min_fee: Option<i32>,
    pub max_fee: Option<i32>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateTariffRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tariff_type: Option<String>,
    pub price_per_kwh: Option<i32>,
    pub price_per_minute: Option<i32>,
    pub session_fee: Option<i32>,
    pub currency: Option<String>,
    pub min_fee: Option<i32>,
    pub max_fee: Option<i32>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CostPreviewRequest {
    pub tariff_id: Option<i32>,
    #[validate(range(min = 0, message = "energy_wh must be non-negative"))]
    pub energy_wh: i32,
    #[validate(range(min = 0, message = "duration_seconds must be non-negative"))]
    pub duration_seconds: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CostBreakdownResponse {
    pub energy_cost: i32,
    pub time_cost: i32,
    pub session_fee: i32,
    pub subtotal: i32,
    pub total: i32,
    pub currency: String,
    pub formatted_total: String,
}
