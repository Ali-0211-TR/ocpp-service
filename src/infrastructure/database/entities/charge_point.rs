//! ChargePoint entity

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "charge_points")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    pub vendor: String,
    pub model: String,

    #[sea_orm(nullable)]
    pub serial_number: Option<String>,

    #[sea_orm(nullable)]
    pub firmware_version: Option<String>,

    #[sea_orm(nullable)]
    pub iccid: Option<String>,

    #[sea_orm(nullable)]
    pub imsi: Option<String>,

    /// Negotiated OCPP protocol version: "V16", "V201", "V21"
    #[sea_orm(nullable)]
    pub ocpp_version: Option<String>,

    /// Meter type (OCPP 1.6 BootNotification optional field)
    #[sea_orm(nullable)]
    pub meter_type: Option<String>,

    /// Meter serial number (OCPP 1.6 BootNotification optional field)
    #[sea_orm(nullable)]
    pub meter_serial_number: Option<String>,

    /// Bcrypt hash of the charge point password (OCPP Security Profile 1 — Basic Auth)
    #[sea_orm(nullable)]
    pub password_hash: Option<String>,

    /// Status: Online, Offline, Unknown
    pub status: String,

    #[sea_orm(nullable)]
    pub last_heartbeat: Option<DateTimeUtc>,

    pub registered_at: DateTimeUtc,

    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::connector::Entity")]
    Connectors,
    #[sea_orm(has_many = "super::transaction::Entity")]
    Transactions,
}

impl Related<super::connector::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Connectors.def()
    }
}

impl Related<super::transaction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Transactions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
