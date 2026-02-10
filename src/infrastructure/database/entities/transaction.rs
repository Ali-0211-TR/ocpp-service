//! Transaction entity with billing support

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Billing status for transactions
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum BillingStatus {
    #[sea_orm(string_value = "Pending")]
    Pending,
    #[sea_orm(string_value = "Calculated")]
    Calculated,
    #[sea_orm(string_value = "Invoiced")]
    Invoiced,
    #[sea_orm(string_value = "Paid")]
    Paid,
    #[sea_orm(string_value = "Failed")]
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

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "transactions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    
    pub charge_point_id: String,
    pub connector_id: i32,
    pub id_tag: String,
    
    pub meter_start: i32,
    
    #[sea_orm(nullable)]
    pub meter_stop: Option<i32>,
    
    pub started_at: DateTimeUtc,
    
    #[sea_orm(nullable)]
    pub stopped_at: Option<DateTimeUtc>,
    
    /// Reason for stopping: EmergencyStop, EVDisconnected, HardReset, Local,
    /// Other, PowerLoss, Reboot, Remote, SoftReset, UnlockCommand, DeAuthorized
    #[sea_orm(nullable)]
    pub stop_reason: Option<String>,
    
    /// Energy consumed in Wh
    #[sea_orm(nullable)]
    pub energy_consumed: Option<i32>,
    
    /// Transaction status: Active, Completed, Invalid
    pub status: String,
    
    // Billing fields
    
    /// Tariff ID used for billing
    #[sea_orm(nullable)]
    pub tariff_id: Option<i32>,
    
    /// Total cost in smallest currency unit (e.g., cents)
    #[sea_orm(nullable)]
    pub total_cost: Option<i32>,
    
    /// Currency code (ISO 4217)
    #[sea_orm(nullable)]
    pub currency: Option<String>,
    
    /// Energy cost component
    #[sea_orm(nullable)]
    pub energy_cost: Option<i32>,
    
    /// Time cost component
    #[sea_orm(nullable)]
    pub time_cost: Option<i32>,
    
    /// Session fee component
    #[sea_orm(nullable)]
    pub session_fee: Option<i32>,
    
    /// Billing status
    #[sea_orm(nullable)]
    pub billing_status: Option<String>,
    
    // Live meter data fields
    
    /// Last meter value reading (Wh)
    #[sea_orm(nullable)]
    pub last_meter_value: Option<i32>,
    
    /// Current charging power (W)
    #[sea_orm(nullable, column_type = "Double")]
    pub current_power_w: Option<f64>,
    
    /// Current State of Charge (%)
    #[sea_orm(nullable)]
    pub current_soc: Option<i32>,
    
    /// Timestamp of last meter values update
    #[sea_orm(nullable)]
    pub last_meter_update: Option<DateTimeUtc>,
    
    // Charging limit fields
    
    /// Limit type: "energy" (kWh), "amount" (cost), "soc" (%)
    #[sea_orm(nullable)]
    pub limit_type: Option<String>,
    
    /// Limit value (kWh for energy, smallest currency unit for amount, % for soc)
    #[sea_orm(nullable, column_type = "Double")]
    pub limit_value: Option<f64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::charge_point::Entity",
        from = "Column::ChargePointId",
        to = "super::charge_point::Column::Id"
    )]
    ChargePoint,
    
    #[sea_orm(
        belongs_to = "super::tariff::Entity",
        from = "Column::TariffId",
        to = "super::tariff::Column::Id"
    )]
    Tariff,
}

impl Related<super::charge_point::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChargePoint.def()
    }
}

impl Related<super::tariff::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tariff.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Calculate duration in seconds
    pub fn duration_seconds(&self) -> Option<i64> {
        self.stopped_at.map(|stop| {
            (stop - self.started_at).num_seconds()
        })
    }
    
    /// Get energy consumed in kWh
    pub fn energy_kwh(&self) -> Option<f64> {
        self.energy_consumed.map(|wh| wh as f64 / 1000.0)
    }
    
    /// Format total cost as string
    pub fn format_cost(&self) -> Option<String> {
        match (self.total_cost, &self.currency) {
            (Some(cost), Some(currency)) => {
                let major = cost / 100;
                let minor = cost % 100;
                Some(format!("{}.{:02} {}", major, minor, currency))
            }
            _ => None,
        }
    }
}
