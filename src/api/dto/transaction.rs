//! Transaction DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::{Transaction, TransactionStatus};

/// Transaction response DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": 1,
    "charge_point_id": "CP001",
    "connector_id": 1,
    "id_tag": "RFID001",
    "meter_start": 10000,
    "meter_stop": 15000,
    "energy_consumed_wh": 5000,
    "status": "Completed",
    "started_at": "2024-01-15T10:00:00Z",
    "stopped_at": "2024-01-15T12:00:00Z",
    "stop_reason": "Local"
}))]
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
}

impl TransactionDto {
    pub fn from_domain(tx: Transaction) -> Self {
        let energy = tx.energy_consumed();
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

/// Filter parameters for transactions
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct TransactionFilter {
    /// Filter by charge point ID
    pub charge_point_id: Option<String>,
    /// Filter by status (active, completed, failed)
    pub status: Option<String>,
    /// Filter transactions started after this date
    pub from_date: Option<DateTime<Utc>>,
    /// Filter transactions started before this date
    pub to_date: Option<DateTime<Utc>>,
}
