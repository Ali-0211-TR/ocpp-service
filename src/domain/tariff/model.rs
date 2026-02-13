//! Tariff domain entity

use chrono::{DateTime, Utc};

/// Tariff type for billing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TariffType {
    /// Flat rate per kWh
    PerKwh,
    /// Flat rate per minute
    PerMinute,
    /// Per session (flat fee)
    PerSession,
    /// Combined (per kWh + per minute + session fee)
    Combined,
}

impl Default for TariffType {
    fn default() -> Self {
        Self::PerKwh
    }
}

impl std::fmt::Display for TariffType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PerKwh => write!(f, "PerKwh"),
            Self::PerMinute => write!(f, "PerMinute"),
            Self::PerSession => write!(f, "PerSession"),
            Self::Combined => write!(f, "Combined"),
        }
    }
}

/// Billing status for transactions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillingStatus {
    Pending,
    Calculated,
    Invoiced,
    Paid,
    Failed,
}

impl Default for BillingStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for BillingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Calculated => write!(f, "Calculated"),
            Self::Invoiced => write!(f, "Invoiced"),
            Self::Paid => write!(f, "Paid"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Tariff for charging sessions
#[derive(Debug, Clone)]
pub struct Tariff {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub tariff_type: TariffType,
    /// Price per kWh (in smallest currency unit, e.g., cents)
    pub price_per_kwh: i32,
    /// Price per minute (in smallest currency unit)
    pub price_per_minute: i32,
    /// Session start fee (in smallest currency unit)
    pub session_fee: i32,
    /// Currency code (ISO 4217)
    pub currency: String,
    /// Minimum charging fee
    pub min_fee: i32,
    /// Maximum charging fee (0 = no limit)
    pub max_fee: i32,
    pub is_active: bool,
    pub is_default: bool,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Tariff {
    /// Calculate cost for a charging session
    ///
    /// # Arguments
    /// * `energy_wh` - Energy consumed in Wh
    /// * `duration_seconds` - Duration in seconds
    ///
    /// # Returns
    /// Total cost in smallest currency unit (e.g., cents)
    pub fn calculate_cost(&self, energy_wh: i32, duration_seconds: i64) -> i32 {
        let energy_kwh = energy_wh as f64 / 1000.0;
        let duration_minutes = duration_seconds as f64 / 60.0;

        let cost = match self.tariff_type {
            TariffType::PerKwh => (energy_kwh * self.price_per_kwh as f64) as i32,
            TariffType::PerMinute => (duration_minutes * self.price_per_minute as f64) as i32,
            TariffType::PerSession => self.session_fee,
            TariffType::Combined => {
                let energy_cost = (energy_kwh * self.price_per_kwh as f64) as i32;
                let time_cost = (duration_minutes * self.price_per_minute as f64) as i32;
                energy_cost + time_cost + self.session_fee
            }
        };

        // Apply min/max constraints
        let cost = cost.max(self.min_fee);
        if self.max_fee > 0 {
            cost.min(self.max_fee)
        } else {
            cost
        }
    }

    /// Calculate detailed cost breakdown
    pub fn calculate_cost_breakdown(&self, energy_wh: i32, duration_seconds: i64) -> CostBreakdown {
        let energy_kwh = energy_wh as f64 / 1000.0;
        let duration_minutes = duration_seconds as f64 / 60.0;

        let energy_cost = (energy_kwh * self.price_per_kwh as f64) as i32;
        let time_cost = (duration_minutes * self.price_per_minute as f64) as i32;
        let session_fee = self.session_fee;

        let subtotal = match self.tariff_type {
            TariffType::PerKwh => energy_cost,
            TariffType::PerMinute => time_cost,
            TariffType::PerSession => session_fee,
            TariffType::Combined => energy_cost + time_cost + session_fee,
        };

        // Apply min/max constraints
        let total = subtotal.max(self.min_fee);
        let total = if self.max_fee > 0 {
            total.min(self.max_fee)
        } else {
            total
        };

        CostBreakdown {
            energy_cost,
            time_cost,
            session_fee,
            subtotal,
            total,
            currency: self.currency.clone(),
        }
    }

    /// Check if tariff is currently valid
    pub fn is_valid(&self) -> bool {
        if !self.is_active {
            return false;
        }

        let now = Utc::now();

        if let Some(valid_from) = self.valid_from {
            if now < valid_from {
                return false;
            }
        }

        if let Some(valid_until) = self.valid_until {
            if now > valid_until {
                return false;
            }
        }

        true
    }

    /// Format cost as human-readable string
    pub fn format_cost(&self, cost_cents: i32) -> String {
        let major = cost_cents / 100;
        let minor = cost_cents % 100;
        format!("{}.{:02} {}", major, minor, self.currency)
    }
}

/// Cost breakdown for a charging session
#[derive(Debug, Clone)]
pub struct CostBreakdown {
    pub energy_cost: i32,
    pub time_cost: i32,
    pub session_fee: i32,
    pub subtotal: i32,
    pub total: i32,
    pub currency: String,
}

impl CostBreakdown {
    pub fn format_total(&self) -> String {
        let major = self.total / 100;
        let minor = self.total % 100;
        format!("{}.{:02} {}", major, minor, self.currency)
    }
}

/// Billing information for a completed transaction
#[derive(Debug, Clone)]
pub struct TransactionBilling {
    pub transaction_id: i32,
    pub tariff_id: Option<i32>,
    pub energy_wh: i32,
    pub duration_seconds: i64,
    pub energy_cost: i32,
    pub time_cost: i32,
    pub session_fee: i32,
    pub total_cost: i32,
    pub currency: String,
    pub status: BillingStatus,
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tariff(tariff_type: TariffType) -> Tariff {
        Tariff {
            id: 1,
            name: "Test".into(),
            description: None,
            tariff_type,
            price_per_kwh: 500,    // 5.00 per kWh
            price_per_minute: 10,  // 0.10 per minute
            session_fee: 100,      // 1.00 flat
            currency: "UZS".into(),
            min_fee: 0,
            max_fee: 0,
            is_active: true,
            is_default: true,
            valid_from: None,
            valid_until: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn calculate_cost_per_kwh() {
        let t = sample_tariff(TariffType::PerKwh);
        // 10 kWh → 10 * 500 = 5000
        assert_eq!(t.calculate_cost(10_000, 3600), 5000);
    }

    #[test]
    fn calculate_cost_per_minute() {
        let t = sample_tariff(TariffType::PerMinute);
        // 60 minutes → 60 * 10 = 600
        assert_eq!(t.calculate_cost(0, 3600), 600);
    }

    #[test]
    fn calculate_cost_per_session() {
        let t = sample_tariff(TariffType::PerSession);
        assert_eq!(t.calculate_cost(50_000, 7200), 100);
    }

    #[test]
    fn calculate_cost_combined() {
        let t = sample_tariff(TariffType::Combined);
        // energy: 10 kWh * 500 = 5000
        // time:   60 min  * 10  = 600
        // session_fee = 100
        // total = 5700
        assert_eq!(t.calculate_cost(10_000, 3600), 5700);
    }

    #[test]
    fn min_fee_is_enforced() {
        let mut t = sample_tariff(TariffType::PerKwh);
        t.min_fee = 1000;
        // 0 kWh → cost=0, but min_fee=1000
        assert_eq!(t.calculate_cost(0, 0), 1000);
    }

    #[test]
    fn max_fee_is_enforced() {
        let mut t = sample_tariff(TariffType::Combined);
        t.max_fee = 2000;
        // normal combined = 5700, but capped at 2000
        assert_eq!(t.calculate_cost(10_000, 3600), 2000);
    }

    #[test]
    fn max_fee_zero_means_unlimited() {
        let mut t = sample_tariff(TariffType::PerKwh);
        t.max_fee = 0;
        assert_eq!(t.calculate_cost(100_000, 0), 50_000);
    }

    #[test]
    fn cost_breakdown_combined() {
        let t = sample_tariff(TariffType::Combined);
        let bd = t.calculate_cost_breakdown(10_000, 3600);
        assert_eq!(bd.energy_cost, 5000);
        assert_eq!(bd.time_cost, 600);
        assert_eq!(bd.session_fee, 100);
        assert_eq!(bd.subtotal, 5700);
        assert_eq!(bd.total, 5700);
        assert_eq!(bd.currency, "UZS");
    }

    #[test]
    fn cost_breakdown_format_total() {
        let t = sample_tariff(TariffType::PerKwh);
        let bd = t.calculate_cost_breakdown(10_000, 3600);
        assert_eq!(bd.format_total(), "50.00 UZS");
    }

    #[test]
    fn format_cost_helper() {
        let t = sample_tariff(TariffType::PerKwh);
        assert_eq!(t.format_cost(12345), "123.45 UZS");
        assert_eq!(t.format_cost(0), "0.00 UZS");
    }

    #[test]
    fn is_valid_when_active_and_no_dates() {
        let t = sample_tariff(TariffType::PerKwh);
        assert!(t.is_valid());
    }

    #[test]
    fn is_valid_when_inactive() {
        let mut t = sample_tariff(TariffType::PerKwh);
        t.is_active = false;
        assert!(!t.is_valid());
    }

    #[test]
    fn is_valid_with_future_valid_from() {
        let mut t = sample_tariff(TariffType::PerKwh);
        t.valid_from = Some(Utc::now() + chrono::Duration::hours(1));
        assert!(!t.is_valid());
    }

    #[test]
    fn is_valid_with_past_valid_until() {
        let mut t = sample_tariff(TariffType::PerKwh);
        t.valid_until = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(!t.is_valid());
    }

    #[test]
    fn tariff_type_display() {
        assert_eq!(TariffType::PerKwh.to_string(), "PerKwh");
        assert_eq!(TariffType::Combined.to_string(), "Combined");
    }

    #[test]
    fn billing_status_display() {
        assert_eq!(BillingStatus::Pending.to_string(), "Pending");
        assert_eq!(BillingStatus::Calculated.to_string(), "Calculated");
    }
}
