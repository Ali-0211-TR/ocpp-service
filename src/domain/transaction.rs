//! Transaction domain entity

use chrono::{DateTime, Utc};

/// Transaction status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionStatus {
    /// Transaction is active
    Active,
    /// Transaction completed successfully
    Completed,
    /// Transaction was stopped with an error
    Failed,
}

/// Charging transaction
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Unique transaction ID
    pub id: i32,
    /// Charge point ID
    pub charge_point_id: String,
    /// Connector ID
    pub connector_id: u32,
    /// ID tag that started the transaction
    pub id_tag: String,
    /// Meter value at start (Wh)
    pub meter_start: i32,
    /// Meter value at stop (Wh)
    pub meter_stop: Option<i32>,
    /// When the transaction started
    pub started_at: DateTime<Utc>,
    /// When the transaction stopped
    pub stopped_at: Option<DateTime<Utc>>,
    /// Stop reason
    pub stop_reason: Option<String>,
    /// Transaction status
    pub status: TransactionStatus,
}

impl Transaction {
    pub fn new(
        id: i32,
        charge_point_id: impl Into<String>,
        connector_id: u32,
        id_tag: impl Into<String>,
        meter_start: i32,
    ) -> Self {
        Self {
            id,
            charge_point_id: charge_point_id.into(),
            connector_id,
            id_tag: id_tag.into(),
            meter_start,
            meter_stop: None,
            started_at: Utc::now(),
            stopped_at: None,
            stop_reason: None,
            status: TransactionStatus::Active,
        }
    }

    pub fn stop(&mut self, meter_stop: i32, reason: Option<String>) {
        self.meter_stop = Some(meter_stop);
        self.stopped_at = Some(Utc::now());
        self.stop_reason = reason;
        self.status = TransactionStatus::Completed;
    }

    /// Calculate energy consumed in Wh
    pub fn energy_consumed(&self) -> Option<i32> {
        self.meter_stop.map(|stop| stop - self.meter_start)
    }

    pub fn is_active(&self) -> bool {
        self.status == TransactionStatus::Active
    }
}
