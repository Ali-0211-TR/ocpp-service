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
            TariffType::PerKwh => {
                (energy_kwh * self.price_per_kwh as f64) as i32
            }
            TariffType::PerMinute => {
                (duration_minutes * self.price_per_minute as f64) as i32
            }
            TariffType::PerSession => {
                self.session_fee
            }
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
        let total = if self.max_fee > 0 { total.min(self.max_fee) } else { total };
        
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
