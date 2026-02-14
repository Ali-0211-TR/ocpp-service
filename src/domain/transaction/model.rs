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

/// Charging limit type
#[derive(Debug, Clone, PartialEq)]
pub enum ChargingLimitType {
    /// Limit by energy in kWh
    Energy,
    /// Limit by cost in smallest currency unit
    Amount,
    /// Limit by SoC percentage
    Soc,
}

impl ChargingLimitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Energy => "energy",
            Self::Amount => "amount",
            Self::Soc => "soc",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "energy" => Some(Self::Energy),
            "amount" => Some(Self::Amount),
            "soc" => Some(Self::Soc),
            _ => None,
        }
    }
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
    /// Last meter value reading (Wh)
    pub last_meter_value: Option<i32>,
    /// Current charging power (W)
    pub current_power_w: Option<f64>,
    /// Current State of Charge (%)
    pub current_soc: Option<i32>,
    /// Timestamp of last meter values update
    pub last_meter_update: Option<DateTime<Utc>>,
    /// Charging limit type
    pub limit_type: Option<ChargingLimitType>,
    /// Charging limit value
    pub limit_value: Option<f64>,
    /// External order ID from integrating system (e.g. GSMS order ID)
    pub external_order_id: Option<String>,
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
            last_meter_value: None,
            current_power_w: None,
            current_soc: None,
            last_meter_update: None,
            limit_type: None,
            limit_value: None,
            external_order_id: None,
        }
    }

    pub fn stop(&mut self, meter_stop: i32, reason: Option<String>) {
        self.meter_stop = Some(meter_stop);
        self.stopped_at = Some(Utc::now());
        self.stop_reason = reason;
        self.status = TransactionStatus::Completed;
    }

    /// Update live meter data
    pub fn update_meter_data(
        &mut self,
        meter_value: Option<i32>,
        power_w: Option<f64>,
        soc: Option<i32>,
    ) {
        if let Some(mv) = meter_value {
            self.last_meter_value = Some(mv);
        }
        if let Some(p) = power_w {
            self.current_power_w = Some(p);
        }
        if let Some(s) = soc {
            self.current_soc = Some(s);
        }
        self.last_meter_update = Some(Utc::now());
    }

    /// Calculate energy consumed in Wh
    pub fn energy_consumed(&self) -> Option<i32> {
        self.meter_stop.map(|stop| stop - self.meter_start)
    }

    /// Calculate live energy consumed in Wh (from last meter value)
    pub fn live_energy_consumed(&self) -> Option<i32> {
        self.last_meter_value.map(|lmv| lmv - self.meter_start)
    }

    pub fn is_active(&self) -> bool {
        self.status == TransactionStatus::Active
    }

    /// Check if the charging limit has been reached
    pub fn is_limit_reached(&self) -> bool {
        match (&self.limit_type, self.limit_value) {
            (Some(ChargingLimitType::Energy), Some(limit_kwh)) => {
                if let Some(energy_wh) = self.live_energy_consumed() {
                    let energy_kwh = energy_wh as f64 / 1000.0;
                    energy_kwh >= limit_kwh
                } else {
                    false
                }
            }
            (Some(ChargingLimitType::Soc), Some(target_soc)) => {
                if let Some(soc) = self.current_soc {
                    soc as f64 >= target_soc
                } else {
                    false
                }
            }
            // Amount limit is checked in billing service
            _ => false,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tx() -> Transaction {
        Transaction::new(1, "CP001", 1, "TAG-001", 1000)
    }

    #[test]
    fn new_transaction_is_active() {
        let tx = sample_tx();
        assert!(tx.is_active());
        assert_eq!(tx.status, TransactionStatus::Active);
        assert_eq!(tx.meter_start, 1000);
        assert!(tx.meter_stop.is_none());
        assert!(tx.stopped_at.is_none());
        assert!(tx.external_order_id.is_none());
    }

    #[test]
    fn new_transaction_with_external_order_id() {
        let mut tx = sample_tx();
        tx.external_order_id = Some("GSMS-ORDER-123".to_string());
        assert_eq!(tx.external_order_id.as_deref(), Some("GSMS-ORDER-123"));
    }

    #[test]
    fn stop_sets_completed() {
        let mut tx = sample_tx();
        tx.stop(5000, Some("Local".into()));
        assert_eq!(tx.status, TransactionStatus::Completed);
        assert_eq!(tx.meter_stop, Some(5000));
        assert_eq!(tx.stop_reason.as_deref(), Some("Local"));
        assert!(tx.stopped_at.is_some());
        assert!(!tx.is_active());
    }

    #[test]
    fn energy_consumed_after_stop() {
        let mut tx = sample_tx();
        tx.stop(6000, None);
        assert_eq!(tx.energy_consumed(), Some(5000));
    }

    #[test]
    fn energy_consumed_none_while_active() {
        let tx = sample_tx();
        assert_eq!(tx.energy_consumed(), None);
    }

    #[test]
    fn live_energy_consumed_from_meter_value() {
        let mut tx = sample_tx();
        tx.update_meter_data(Some(3000), None, None);
        assert_eq!(tx.live_energy_consumed(), Some(2000));
    }

    #[test]
    fn update_meter_data() {
        let mut tx = sample_tx();
        tx.update_meter_data(Some(2000), Some(7500.0), Some(45));
        assert_eq!(tx.last_meter_value, Some(2000));
        assert_eq!(tx.current_power_w, Some(7500.0));
        assert_eq!(tx.current_soc, Some(45));
        assert!(tx.last_meter_update.is_some());
    }

    #[test]
    fn energy_limit_reached() {
        let mut tx = sample_tx();
        tx.limit_type = Some(ChargingLimitType::Energy);
        tx.limit_value = Some(5.0); // 5 kWh
        tx.last_meter_value = Some(6000); // consumed 5000 Wh = 5 kWh
        assert!(tx.is_limit_reached());
    }

    #[test]
    fn energy_limit_not_reached() {
        let mut tx = sample_tx();
        tx.limit_type = Some(ChargingLimitType::Energy);
        tx.limit_value = Some(10.0);
        tx.last_meter_value = Some(3000); // 2 kWh < 10 kWh
        assert!(!tx.is_limit_reached());
    }

    #[test]
    fn soc_limit_reached() {
        let mut tx = sample_tx();
        tx.limit_type = Some(ChargingLimitType::Soc);
        tx.limit_value = Some(80.0);
        tx.current_soc = Some(80);
        assert!(tx.is_limit_reached());
    }

    #[test]
    fn no_limit_means_not_reached() {
        let tx = sample_tx();
        assert!(!tx.is_limit_reached());
    }

    #[test]
    fn charging_limit_type_roundtrip() {
        for lt in &[
            ChargingLimitType::Energy,
            ChargingLimitType::Amount,
            ChargingLimitType::Soc,
        ] {
            let s = lt.as_str();
            let parsed = ChargingLimitType::from_str(s).unwrap();
            assert_eq!(&parsed, lt);
        }
        assert!(ChargingLimitType::from_str("unknown").is_none());
    }
}
