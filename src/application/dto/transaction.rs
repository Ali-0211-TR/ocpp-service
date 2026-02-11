//! Transaction DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::{Transaction, TransactionStatus};

/// Transaction (charging session) DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionDto {
    pub id: i32,
    pub charge_point_id: String,
    pub connector_id: u32,
    pub id_tag: String,
    pub meter_start: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_stop: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_consumed_wh: Option<i32>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_meter_value: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_power_w: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_soc: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_meter_update: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_value: Option<f64>,
}

impl TransactionDto {
    pub fn from_domain(tx: Transaction) -> Self {
        let energy = tx.energy_consumed().or_else(|| tx.live_energy_consumed());
        Self {
            id: tx.id,
            charge_point_id: tx.charge_point_id,
            connector_id: tx.connector_id,
            id_tag: tx.id_tag,
            meter_start: tx.meter_start,
            meter_stop: tx.meter_stop,
            energy_consumed_wh: energy,
            status: transaction_status_to_string(&tx.status),
            started_at: tx.started_at,
            stopped_at: tx.stopped_at,
            stop_reason: tx.stop_reason,
            last_meter_value: tx.last_meter_value,
            current_power_w: tx.current_power_w,
            current_soc: tx.current_soc,
            last_meter_update: tx.last_meter_update,
            limit_type: tx.limit_type.as_ref().map(|lt| lt.as_str().to_string()),
            limit_value: tx.limit_value,
        }
    }
}

fn transaction_status_to_string(status: &TransactionStatus) -> String {
    match status {
        TransactionStatus::Active => "Active",
        TransactionStatus::Completed => "Completed",
        TransactionStatus::Failed => "Failed",
    }
    .to_string()
}

/// Transaction query filters
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct TransactionFilter {
    pub charge_point_id: Option<String>,
    pub status: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
}
