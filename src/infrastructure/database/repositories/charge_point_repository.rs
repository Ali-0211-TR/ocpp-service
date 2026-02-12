//! SeaORM implementation of ChargePointRepository

use async_trait::async_trait;
use chrono::Utc;
use log::{debug, info};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, Set,
};

use crate::domain::charge_point::{
    ChargePoint, ChargePointRepository, ChargePointStatus, Connector, ConnectorStatus,
};
use crate::domain::OcppVersion;
use crate::domain::{DomainError, DomainResult};
use crate::infrastructure::database::entities::{charge_point, connector};

pub struct SeaOrmChargePointRepository {
    db: DatabaseConnection,
}

impl SeaOrmChargePointRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Conversion helpers ──────────────────────────────────────────

fn status_to_string(status: &ChargePointStatus) -> String {
    status.to_string()
}

fn string_to_status(s: &str) -> ChargePointStatus {
    ChargePointStatus::from(s)
}

fn connector_status_to_string(status: &ConnectorStatus) -> String {
    match status {
        ConnectorStatus::Available => "Available",
        ConnectorStatus::Preparing => "Preparing",
        ConnectorStatus::Charging => "Charging",
        ConnectorStatus::SuspendedEV => "SuspendedEV",
        ConnectorStatus::SuspendedEVSE => "SuspendedEVSE",
        ConnectorStatus::Finishing => "Finishing",
        ConnectorStatus::Reserved => "Reserved",
        ConnectorStatus::Unavailable => "Unavailable",
        ConnectorStatus::Faulted => "Faulted",
    }
    .to_string()
}

fn string_to_connector_status(s: &str) -> ConnectorStatus {
    match s {
        "Available" => ConnectorStatus::Available,
        "Preparing" => ConnectorStatus::Preparing,
        "Charging" => ConnectorStatus::Charging,
        "SuspendedEV" => ConnectorStatus::SuspendedEV,
        "SuspendedEVSE" => ConnectorStatus::SuspendedEVSE,
        "Finishing" => ConnectorStatus::Finishing,
        "Reserved" => ConnectorStatus::Reserved,
        "Unavailable" => ConnectorStatus::Unavailable,
        "Faulted" => ConnectorStatus::Faulted,
        _ => ConnectorStatus::Unavailable,
    }
}

fn db_err(e: sea_orm::DbErr) -> DomainError {
    DomainError::Validation(format!("Database error: {}", e))
}

fn ocpp_version_to_string(v: &OcppVersion) -> String {
    match v {
        OcppVersion::V16 => "V16".to_string(),
        OcppVersion::V201 => "V201".to_string(),
        OcppVersion::V21 => "V21".to_string(),
    }
}

fn string_to_ocpp_version(s: &str) -> Option<OcppVersion> {
    match s {
        "V16" => Some(OcppVersion::V16),
        "V201" => Some(OcppVersion::V201),
        "V21" => Some(OcppVersion::V21),
        _ => None,
    }
}

fn connectors_from_models(models: Vec<connector::Model>) -> Vec<Connector> {
    models
        .into_iter()
        .map(|c| Connector {
            id: c.connector_id as u32,
            status: string_to_connector_status(&c.status),
            error_code: c.error_code,
            info: c.error_info,
            vendor_id: c.vendor_id,
            vendor_error_code: c.vendor_error_code,
        })
        .collect()
}

fn cp_from_model(model: charge_point::Model, connectors: Vec<Connector>) -> ChargePoint {
    ChargePoint {
        id: model.id,
        ocpp_version: model.ocpp_version.as_deref().and_then(string_to_ocpp_version),
        vendor: Some(model.vendor),
        model: Some(model.model),
        serial_number: model.serial_number,
        firmware_version: model.firmware_version,
        iccid: model.iccid,
        imsi: model.imsi,
        meter_type: model.meter_type,
        meter_serial_number: model.meter_serial_number,
        status: string_to_status(&model.status),
        connectors,
        registered_at: model.registered_at,
        last_heartbeat: model.last_heartbeat,
    }
}

async fn load_connectors(
    db: &DatabaseConnection,
    charge_point_id: &str,
) -> DomainResult<Vec<Connector>> {
    let models = connector::Entity::find()
        .filter(connector::Column::ChargePointId.eq(charge_point_id))
        .all(db)
        .await
        .map_err(db_err)?;
    Ok(connectors_from_models(models))
}

async fn save_connectors(
    db: &DatabaseConnection,
    charge_point_id: &str,
    connectors: &[Connector],
) -> DomainResult<()> {
    for conn in connectors {
        let model = connector::ActiveModel {
            id: NotSet,
            charge_point_id: Set(charge_point_id.to_string()),
            connector_id: Set(conn.id as i32),
            status: Set(connector_status_to_string(&conn.status)),
            error_code: Set(conn.error_code.clone()),
            error_info: Set(conn.info.clone()),
            vendor_id: Set(conn.vendor_id.clone()),
            vendor_error_code: Set(conn.vendor_error_code.clone()),
            updated_at: Set(Utc::now()),
        };
        model.insert(db).await.map_err(db_err)?;
    }
    Ok(())
}

async fn delete_connectors(db: &DatabaseConnection, charge_point_id: &str) -> DomainResult<()> {
    connector::Entity::delete_many()
        .filter(connector::Column::ChargePointId.eq(charge_point_id))
        .exec(db)
        .await
        .map_err(db_err)?;
    Ok(())
}

// ── ChargePointRepository impl ──────────────────────────────────

#[async_trait]
impl ChargePointRepository for SeaOrmChargePointRepository {
    async fn save(&self, cp: ChargePoint) -> DomainResult<()> {
        debug!("Saving charge point: {}", cp.id);

        let existing = charge_point::Entity::find_by_id(&cp.id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        if existing.is_some() {
            return Err(DomainError::Conflict(format!(
                "Charge point '{}' already exists",
                cp.id
            )));
        }

        let model = charge_point::ActiveModel {
            id: Set(cp.id.clone()),
            vendor: Set(cp.vendor.unwrap_or_default()),
            model: Set(cp.model.unwrap_or_default()),
            serial_number: Set(cp.serial_number),
            firmware_version: Set(cp.firmware_version),
            iccid: Set(cp.iccid),
            imsi: Set(cp.imsi),
            ocpp_version: Set(cp.ocpp_version.as_ref().map(ocpp_version_to_string)),
            meter_type: Set(cp.meter_type),
            meter_serial_number: Set(cp.meter_serial_number),
            status: Set(status_to_string(&cp.status)),
            last_heartbeat: Set(cp.last_heartbeat),
            registered_at: Set(cp.registered_at),
            updated_at: Set(Some(Utc::now())),
        };
        model.insert(&self.db).await.map_err(db_err)?;

        save_connectors(&self.db, &cp.id, &cp.connectors).await?;

        info!("Charge point saved: {}", cp.id);
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> DomainResult<Option<ChargePoint>> {
        let model = charge_point::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        let Some(model) = model else {
            return Ok(None);
        };

        let connectors = load_connectors(&self.db, id).await?;
        Ok(Some(cp_from_model(model, connectors)))
    }

    async fn find_all(&self) -> DomainResult<Vec<ChargePoint>> {
        let models = charge_point::Entity::find()
            .all(&self.db)
            .await
            .map_err(db_err)?;

        let mut result = Vec::with_capacity(models.len());
        for model in models {
            let connectors = load_connectors(&self.db, &model.id).await?;
            result.push(cp_from_model(model, connectors));
        }
        Ok(result)
    }

    async fn update(&self, cp: ChargePoint) -> DomainResult<()> {
        debug!("Updating charge point: {}", cp.id);

        let existing = charge_point::Entity::find_by_id(&cp.id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        if existing.is_none() {
            return Err(DomainError::NotFound {
                entity: "ChargePoint",
                field: "id",
                value: cp.id,
            });
        }

        let model = charge_point::ActiveModel {
            id: Set(cp.id.clone()),
            vendor: Set(cp.vendor.unwrap_or_default()),
            model: Set(cp.model.unwrap_or_default()),
            serial_number: Set(cp.serial_number),
            firmware_version: Set(cp.firmware_version),
            iccid: Set(cp.iccid),
            imsi: Set(cp.imsi),
            ocpp_version: Set(cp.ocpp_version.as_ref().map(ocpp_version_to_string)),
            meter_type: Set(cp.meter_type),
            meter_serial_number: Set(cp.meter_serial_number),
            status: Set(status_to_string(&cp.status)),
            last_heartbeat: Set(cp.last_heartbeat),
            registered_at: Set(cp.registered_at),
            updated_at: Set(Some(Utc::now())),
        };
        model.update(&self.db).await.map_err(db_err)?;

        delete_connectors(&self.db, &cp.id).await?;
        save_connectors(&self.db, &cp.id, &cp.connectors).await?;

        Ok(())
    }

    async fn update_status(&self, id: &str, status: ChargePointStatus) -> DomainResult<()> {
        debug!("Updating charge point status: {} -> {:?}", id, status);

        let existing = charge_point::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        if existing.is_none() {
            return Err(DomainError::NotFound {
                entity: "ChargePoint",
                field: "id",
                value: id.to_string(),
            });
        }

        let model = charge_point::ActiveModel {
            id: Set(id.to_string()),
            status: Set(status_to_string(&status)),
            updated_at: Set(Some(Utc::now())),
            vendor: NotSet,
            model: NotSet,
            serial_number: NotSet,
            firmware_version: NotSet,
            iccid: NotSet,
            imsi: NotSet,
            ocpp_version: NotSet,
            meter_type: NotSet,
            meter_serial_number: NotSet,
            last_heartbeat: NotSet,
            registered_at: NotSet,
        };
        model.update(&self.db).await.map_err(db_err)?;

        info!("Charge point {} status updated to {:?}", id, status);
        Ok(())
    }

    async fn delete(&self, id: &str) -> DomainResult<()> {
        let result = charge_point::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(db_err)?;

        if result.rows_affected == 0 {
            return Err(DomainError::NotFound {
                entity: "ChargePoint",
                field: "id",
                value: id.to_string(),
            });
        }
        Ok(())
    }
}
