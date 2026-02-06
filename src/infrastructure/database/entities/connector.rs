//! Connector entity

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "connectors")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    
    pub charge_point_id: String,
    
    /// Connector number on the charge point (1, 2, etc.)
    pub connector_id: i32,
    
    /// OCPP ChargePointStatus: Available, Preparing, Charging, SuspendedEVSE, 
    /// SuspendedEV, Finishing, Reserved, Unavailable, Faulted
    pub status: String,
    
    /// OCPP ChargePointErrorCode
    #[sea_orm(nullable)]
    pub error_code: Option<String>,
    
    #[sea_orm(nullable)]
    pub error_info: Option<String>,
    
    #[sea_orm(nullable)]
    pub vendor_id: Option<String>,
    
    #[sea_orm(nullable)]
    pub vendor_error_code: Option<String>,
    
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
