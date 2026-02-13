//! ChargingProfile entity

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "charging_profiles")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub charge_point_id: String,

    /// EVSE / connector ID (0 = station-wide).
    pub evse_id: i32,

    /// Profile ID from the OCPP ChargingProfile object.
    pub profile_id: i32,

    pub stack_level: i32,

    /// ChargingProfilePurpose: TxDefaultProfile, TxProfile, ChargingStationMaxProfile, etc.
    pub purpose: String,

    /// ChargingProfileKind: Absolute, Recurring, Relative.
    pub kind: String,

    /// RecurrencyKind: Daily, Weekly (nullable).
    #[sea_orm(nullable)]
    pub recurrency_kind: Option<String>,

    #[sea_orm(nullable)]
    pub valid_from: Option<DateTimeUtc>,

    #[sea_orm(nullable)]
    pub valid_to: Option<DateTimeUtc>,

    /// Full charging schedule(s) as JSON.
    #[sea_orm(column_type = "Text")]
    pub schedule_json: String,

    /// Whether this profile is still active on the charge point.
    pub is_active: bool,

    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::charge_point::Entity",
        from = "Column::ChargePointId",
        to = "super::charge_point::Column::Id"
    )]
    ChargePoint,
}

impl Related<super::charge_point::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChargePoint.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
