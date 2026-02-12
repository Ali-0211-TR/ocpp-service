//! Tariff entity for billing

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Tariff type
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum TariffType {
    /// Flat rate per kWh
    #[sea_orm(string_value = "PerKwh")]
    PerKwh,
    /// Flat rate per minute
    #[sea_orm(string_value = "PerMinute")]
    PerMinute,
    /// Per session (flat fee)
    #[sea_orm(string_value = "PerSession")]
    PerSession,
    /// Combined (per kWh + per minute + session fee)
    #[sea_orm(string_value = "Combined")]
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

/// Tariff model - defines pricing for charging sessions
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tariffs")]
pub struct Model {
    /// Unique tariff ID
    #[sea_orm(primary_key)]
    pub id: i32,

    /// Tariff name (e.g., "Standard", "Premium", "Night Rate")
    pub name: String,

    /// Description of the tariff
    pub description: Option<String>,

    /// Tariff type
    pub tariff_type: TariffType,

    /// Price per kWh (in smallest currency unit, e.g., cents)
    pub price_per_kwh: i32,

    /// Price per minute (in smallest currency unit)
    pub price_per_minute: i32,

    /// Session start fee (in smallest currency unit)
    pub session_fee: i32,

    /// Currency code (ISO 4217, e.g., "USD", "EUR", "UZS")
    pub currency: String,

    /// Minimum charging fee
    pub min_fee: i32,

    /// Maximum charging fee (0 = no limit)
    pub max_fee: i32,

    /// Whether this tariff is active
    pub is_active: bool,

    /// Whether this is the default tariff
    pub is_default: bool,

    /// Valid from date (optional)
    pub valid_from: Option<DateTime<Utc>>,

    /// Valid until date (optional)
    pub valid_until: Option<DateTime<Utc>>,

    /// When the tariff was created
    pub created_at: DateTime<Utc>,

    /// When the tariff was last updated
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
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
