//! Database storage implementation using SeaORM

use async_trait::async_trait;
use chrono::Utc;
use log::{debug, info};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, Set,
};

use super::entities::{charge_point, connector, id_tag, tariff, transaction};
use crate::domain::{
    BillingStatus, ChargePoint, ChargePointStatus, ChargingLimitType, Connector, ConnectorStatus, DomainError, DomainResult,
    Tariff, TariffType, Transaction, TransactionBilling, TransactionStatus,
};
use crate::domain::Storage;

/// Database storage implementation
pub struct DatabaseStorage {
    db: DatabaseConnection,
}

impl DatabaseStorage {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Get database connection reference
    pub fn connection(&self) -> &DatabaseConnection {
        &self.db
    }
}

// Helper functions for domain <-> entity conversion

fn domain_charge_point_status_to_string(status: &ChargePointStatus) -> String {
    status.to_string()
}

fn string_to_domain_charge_point_status(s: &str) -> ChargePointStatus {
    ChargePointStatus::from(s)
}

fn domain_connector_status_to_string(status: &ConnectorStatus) -> String {
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

fn string_to_domain_connector_status(s: &str) -> ConnectorStatus {
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

fn domain_transaction_status_to_string(status: &TransactionStatus) -> String {
    match status {
        TransactionStatus::Active => "Active",
        TransactionStatus::Completed => "Completed",
        TransactionStatus::Failed => "Failed",
    }
    .to_string()
}

fn string_to_domain_transaction_status(s: &str) -> TransactionStatus {
    match s {
        "Active" => TransactionStatus::Active,
        "Completed" => TransactionStatus::Completed,
        "Failed" => TransactionStatus::Failed,
        _ => TransactionStatus::Failed,
    }
}

fn tariff_entity_to_domain(t: tariff::Model) -> Tariff {
    Tariff {
        id: t.id,
        name: t.name,
        description: t.description,
        tariff_type: match t.tariff_type {
            tariff::TariffType::PerKwh => TariffType::PerKwh,
            tariff::TariffType::PerMinute => TariffType::PerMinute,
            tariff::TariffType::PerSession => TariffType::PerSession,
            tariff::TariffType::Combined => TariffType::Combined,
        },
        price_per_kwh: t.price_per_kwh,
        price_per_minute: t.price_per_minute,
        session_fee: t.session_fee,
        currency: t.currency,
        min_fee: t.min_fee,
        max_fee: t.max_fee,
        is_active: t.is_active,
        is_default: t.is_default,
        valid_from: t.valid_from,
        valid_until: t.valid_until,
        created_at: t.created_at,
        updated_at: t.updated_at,
    }
}

fn domain_tariff_type_to_entity(t: &TariffType) -> tariff::TariffType {
    match t {
        TariffType::PerKwh => tariff::TariffType::PerKwh,
        TariffType::PerMinute => tariff::TariffType::PerMinute,
        TariffType::PerSession => tariff::TariffType::PerSession,
        TariffType::Combined => tariff::TariffType::Combined,
    }
}

fn string_to_billing_status(s: &str) -> BillingStatus {
    match s {
        "Pending" => BillingStatus::Pending,
        "Calculated" => BillingStatus::Calculated,
        "Invoiced" => BillingStatus::Invoiced,
        "Paid" => BillingStatus::Paid,
        "Failed" => BillingStatus::Failed,
        _ => BillingStatus::Pending,
    }
}

fn db_error_to_domain(e: DbErr) -> DomainError {
    DomainError::Validation(format!("Database error: {}", e))
}

fn transaction_model_to_domain(t: transaction::Model) -> Transaction {
    Transaction {
        id: t.id,
        charge_point_id: t.charge_point_id,
        connector_id: t.connector_id as u32,
        id_tag: t.id_tag,
        meter_start: t.meter_start,
        meter_stop: t.meter_stop,
        started_at: t.started_at,
        stopped_at: t.stopped_at,
        stop_reason: t.stop_reason,
        status: string_to_domain_transaction_status(&t.status),
        last_meter_value: t.last_meter_value,
        current_power_w: t.current_power_w,
        current_soc: t.current_soc,
        last_meter_update: t.last_meter_update,
        limit_type: t.limit_type.as_deref().and_then(ChargingLimitType::from_str),
        limit_value: t.limit_value,
    }
}

#[async_trait]
impl Storage for DatabaseStorage {
    async fn save_charge_point(&self, cp: ChargePoint) -> DomainResult<()> {
        debug!("Saving charge point: {}", cp.id);

        // Check if already exists
        let existing = charge_point::Entity::find_by_id(&cp.id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        if existing.is_some() {
            return Err(DomainError::Conflict(format!("Charge point '{}' already exists", cp.id)));
        }

        // Insert charge point
        let model = charge_point::ActiveModel {
            id: Set(cp.id.clone()),
            vendor: Set(cp.vendor.unwrap_or_default()),
            model: Set(cp.model.unwrap_or_default()),
            serial_number: Set(cp.serial_number),
            firmware_version: Set(cp.firmware_version),
            iccid: Set(cp.iccid),
            imsi: Set(cp.imsi),
            status: Set(domain_charge_point_status_to_string(&cp.status)),
            last_heartbeat: Set(cp.last_heartbeat),
            registered_at: Set(cp.registered_at),
            updated_at: Set(Some(Utc::now())),
        };

        model.insert(&self.db).await.map_err(db_error_to_domain)?;

        // Insert connectors
        for conn in &cp.connectors {
            let connector_model = connector::ActiveModel {
                id: NotSet,
                charge_point_id: Set(cp.id.clone()),
                connector_id: Set(conn.id as i32),
                status: Set(domain_connector_status_to_string(&conn.status)),
                error_code: Set(conn.error_code.clone()),
                error_info: Set(conn.info.clone()),
                vendor_id: Set(conn.vendor_id.clone()),
                vendor_error_code: Set(conn.vendor_error_code.clone()),
                updated_at: Set(Utc::now()),
            };
            connector_model
                .insert(&self.db)
                .await
                .map_err(db_error_to_domain)?;
        }

        info!("Charge point saved: {}", cp.id);
        Ok(())
    }

    async fn get_charge_point(&self, id: &str) -> DomainResult<Option<ChargePoint>> {
        let cp_model = charge_point::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        let Some(cp_model) = cp_model else {
            return Ok(None);
        };

        // Load connectors
        let connectors_models = connector::Entity::find()
            .filter(connector::Column::ChargePointId.eq(id))
            .all(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        let connectors: Vec<Connector> = connectors_models
            .into_iter()
            .map(|c| Connector {
                id: c.connector_id as u32,
                status: string_to_domain_connector_status(&c.status),
                error_code: c.error_code,
                info: c.error_info,
                vendor_id: c.vendor_id,
                vendor_error_code: c.vendor_error_code,
            })
            .collect();

        let cp = ChargePoint {
            id: cp_model.id,
            vendor: Some(cp_model.vendor),
            model: Some(cp_model.model),
            serial_number: cp_model.serial_number,
            firmware_version: cp_model.firmware_version,
            iccid: cp_model.iccid,
            imsi: cp_model.imsi,
            meter_type: None,
            meter_serial_number: None,
            status: string_to_domain_charge_point_status(&cp_model.status),
            connectors,
            registered_at: cp_model.registered_at,
            last_heartbeat: cp_model.last_heartbeat,
        };

        Ok(Some(cp))
    }

    async fn update_charge_point(&self, cp: ChargePoint) -> DomainResult<()> {
        debug!("Updating charge point: {}", cp.id);

        let existing = charge_point::Entity::find_by_id(&cp.id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        if existing.is_none() {
            return Err(DomainError::NotFound { entity: "ChargePoint", field: "id", value: cp.id });
        }

        // Update charge point
        let model = charge_point::ActiveModel {
            id: Set(cp.id.clone()),
            vendor: Set(cp.vendor.unwrap_or_default()),
            model: Set(cp.model.unwrap_or_default()),
            serial_number: Set(cp.serial_number),
            firmware_version: Set(cp.firmware_version),
            iccid: Set(cp.iccid),
            imsi: Set(cp.imsi),
            status: Set(domain_charge_point_status_to_string(&cp.status)),
            last_heartbeat: Set(cp.last_heartbeat),
            registered_at: Set(cp.registered_at),
            updated_at: Set(Some(Utc::now())),
        };

        model.update(&self.db).await.map_err(db_error_to_domain)?;

        // Update connectors - delete existing and insert new
        connector::Entity::delete_many()
            .filter(connector::Column::ChargePointId.eq(&cp.id))
            .exec(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        for conn in &cp.connectors {
            let connector_model = connector::ActiveModel {
                id: NotSet,
                charge_point_id: Set(cp.id.clone()),
                connector_id: Set(conn.id as i32),
                status: Set(domain_connector_status_to_string(&conn.status)),
                error_code: Set(conn.error_code.clone()),
                error_info: Set(conn.info.clone()),
                vendor_id: Set(conn.vendor_id.clone()),
                vendor_error_code: Set(conn.vendor_error_code.clone()),
                updated_at: Set(Utc::now()),
            };
            connector_model
                .insert(&self.db)
                .await
                .map_err(db_error_to_domain)?;
        }

        Ok(())
    }

    async fn delete_charge_point(&self, id: &str) -> DomainResult<()> {
        let result = charge_point::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        if result.rows_affected == 0 {
            return Err(DomainError::NotFound { entity: "ChargePoint", field: "id", value: id.to_string() });
        }

        Ok(())
    }

    async fn list_charge_points(&self) -> DomainResult<Vec<ChargePoint>> {
        let cp_models = charge_point::Entity::find()
            .all(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        let mut result = Vec::new();

        for cp_model in cp_models {
            // Load connectors for each charge point
            let connectors_models = connector::Entity::find()
                .filter(connector::Column::ChargePointId.eq(&cp_model.id))
                .all(&self.db)
                .await
                .map_err(db_error_to_domain)?;

            let connectors: Vec<Connector> = connectors_models
                .into_iter()
                .map(|c| Connector {
                    id: c.connector_id as u32,
                    status: string_to_domain_connector_status(&c.status),
                    error_code: c.error_code,
                    info: c.error_info,
                    vendor_id: c.vendor_id,
                    vendor_error_code: c.vendor_error_code,
                })
                .collect();

            result.push(ChargePoint {
                id: cp_model.id,
                vendor: Some(cp_model.vendor),
                model: Some(cp_model.model),
                serial_number: cp_model.serial_number,
                firmware_version: cp_model.firmware_version,
                iccid: cp_model.iccid,
                imsi: cp_model.imsi,
                meter_type: None,
                meter_serial_number: None,
                status: string_to_domain_charge_point_status(&cp_model.status),
                connectors,
                registered_at: cp_model.registered_at,
                last_heartbeat: cp_model.last_heartbeat,
            });
        }

        Ok(result)
    }

    async fn update_charge_point_status(&self, id: &str, status: ChargePointStatus) -> DomainResult<()> {
        use sea_orm::ActiveValue::NotSet;
        
        debug!("Updating charge point status: {} -> {:?}", id, status);

        let existing = charge_point::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        if existing.is_none() {
            return Err(DomainError::NotFound { entity: "ChargePoint", field: "id", value: id.to_string() });
        }

        // Update only status and updated_at
        let model = charge_point::ActiveModel {
            id: Set(id.to_string()),
            status: Set(domain_charge_point_status_to_string(&status)),
            updated_at: Set(Some(Utc::now())),
            // Keep other fields unchanged
            vendor: NotSet,
            model: NotSet,
            serial_number: NotSet,
            firmware_version: NotSet,
            iccid: NotSet,
            imsi: NotSet,
            last_heartbeat: NotSet,
            registered_at: NotSet,
        };

        model.update(&self.db).await.map_err(db_error_to_domain)?;

        info!("Charge point {} status updated to {:?}", id, status);
        Ok(())
    }

    async fn save_transaction(&self, tx: Transaction) -> DomainResult<()> {
        debug!("Saving transaction: {}", tx.id);

        let energy = tx.energy_consumed();

        let model = transaction::ActiveModel {
            id: Set(tx.id),
            charge_point_id: Set(tx.charge_point_id),
            connector_id: Set(tx.connector_id as i32),
            id_tag: Set(tx.id_tag),
            meter_start: Set(tx.meter_start),
            meter_stop: Set(tx.meter_stop),
            started_at: Set(tx.started_at),
            stopped_at: Set(tx.stopped_at),
            stop_reason: Set(tx.stop_reason),
            energy_consumed: Set(energy),
            status: Set(domain_transaction_status_to_string(&tx.status)),
            // Billing fields - initially null
            tariff_id: Set(None),
            total_cost: Set(None),
            currency: Set(None),
            energy_cost: Set(None),
            time_cost: Set(None),
            session_fee: Set(None),
            billing_status: Set(Some("Pending".to_string())),
            // Live meter data
            last_meter_value: Set(tx.last_meter_value),
            current_power_w: Set(tx.current_power_w),
            current_soc: Set(tx.current_soc),
            last_meter_update: Set(tx.last_meter_update),
            // Charging limits
            limit_type: Set(tx.limit_type.as_ref().map(|lt| lt.as_str().to_string())),
            limit_value: Set(tx.limit_value),
        };

        model.insert(&self.db).await.map_err(db_error_to_domain)?;
        info!("Transaction saved: {}", tx.id);
        Ok(())
    }

    async fn get_transaction(&self, id: i32) -> DomainResult<Option<Transaction>> {
        let tx_model = transaction::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        Ok(tx_model.map(transaction_model_to_domain))
    }

    async fn update_transaction(&self, tx: Transaction) -> DomainResult<()> {
        debug!("Updating transaction: {}", tx.id);

        let existing = transaction::Entity::find_by_id(tx.id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        let Some(existing) = existing else {
            return Err(DomainError::NotFound { entity: "Transaction", field: "id", value: tx.id.to_string() });
        };

        let energy = tx.energy_consumed();

        // Keep existing billing fields when updating transaction data
        let model = transaction::ActiveModel {
            id: Set(tx.id),
            charge_point_id: Set(tx.charge_point_id),
            connector_id: Set(tx.connector_id as i32),
            id_tag: Set(tx.id_tag),
            meter_start: Set(tx.meter_start),
            meter_stop: Set(tx.meter_stop),
            started_at: Set(tx.started_at),
            stopped_at: Set(tx.stopped_at),
            stop_reason: Set(tx.stop_reason),
            energy_consumed: Set(energy),
            status: Set(domain_transaction_status_to_string(&tx.status)),
            // Preserve existing billing fields
            tariff_id: Set(existing.tariff_id),
            total_cost: Set(existing.total_cost),
            currency: Set(existing.currency),
            energy_cost: Set(existing.energy_cost),
            time_cost: Set(existing.time_cost),
            session_fee: Set(existing.session_fee),
            billing_status: Set(existing.billing_status),
            // Live meter data
            last_meter_value: Set(tx.last_meter_value),
            current_power_w: Set(tx.current_power_w),
            current_soc: Set(tx.current_soc),
            last_meter_update: Set(tx.last_meter_update),
            // Charging limits
            limit_type: Set(tx.limit_type.as_ref().map(|lt| lt.as_str().to_string())),
            limit_value: Set(tx.limit_value),
        };

        model.update(&self.db).await.map_err(db_error_to_domain)?;
        Ok(())
    }

    async fn get_active_transaction_for_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> DomainResult<Option<Transaction>> {
        let tx_model = transaction::Entity::find()
            .filter(transaction::Column::ChargePointId.eq(charge_point_id))
            .filter(transaction::Column::ConnectorId.eq(connector_id as i32))
            .filter(transaction::Column::Status.eq("Active"))
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        Ok(tx_model.map(transaction_model_to_domain))
    }

    async fn list_transactions_for_charge_point(
        &self,
        charge_point_id: &str,
    ) -> DomainResult<Vec<Transaction>> {
        let tx_models = transaction::Entity::find()
            .filter(transaction::Column::ChargePointId.eq(charge_point_id))
            .all(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        Ok(tx_models.into_iter().map(transaction_model_to_domain).collect())
    }

    async fn list_all_transactions(&self) -> DomainResult<Vec<Transaction>> {
        let tx_models = transaction::Entity::find()
            .order_by_desc(transaction::Column::Id)
            .all(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        Ok(tx_models.into_iter().map(transaction_model_to_domain).collect())
    }

    async fn update_transaction_meter_data(
        &self,
        transaction_id: i32,
        meter_value: Option<i32>,
        power_w: Option<f64>,
        soc: Option<i32>,
    ) -> DomainResult<()> {
        debug!("Updating meter data for transaction {}: meter={:?}, power={:?}, soc={:?}", 
            transaction_id, meter_value, power_w, soc);

        let existing = transaction::Entity::find_by_id(transaction_id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;

        let Some(existing) = existing else {
            return Err(DomainError::NotFound { entity: "Transaction", field: "id", value: transaction_id.to_string() });
        };

        let mut active: transaction::ActiveModel = existing.into();
        
        if let Some(mv) = meter_value {
            active.last_meter_value = Set(Some(mv));
        }
        if let Some(p) = power_w {
            active.current_power_w = Set(Some(p));
        }
        if let Some(s) = soc {
            active.current_soc = Set(Some(s));
        }
        active.last_meter_update = Set(Some(Utc::now()));

        active.update(&self.db).await.map_err(db_error_to_domain)?;
        Ok(())
    }

    async fn is_id_tag_valid(&self, id_tag_value: &str) -> DomainResult<bool> {
        if id_tag_value.is_empty() {
            return Ok(false);
        }

        // Look up the ID tag in database
        let tag = id_tag::Entity::find_by_id(id_tag_value)
            .one(&self.db)
            .await
            .map_err(|e| DomainError::Validation(format!("Database error: {}", e)))?;

        match tag {
            Some(t) => {
                // Check if valid and update last_used_at
                let is_valid = t.is_valid();
                
                if is_valid {
                    // Update last_used_at timestamp
                    let mut active: id_tag::ActiveModel = t.into();
                    active.last_used_at = Set(Some(Utc::now()));
                    let _ = active.update(&self.db).await;
                }
                
                Ok(is_valid)
            }
            None => {
                // Tag not found - invalid
                debug!("IdTag '{}' not found in database", id_tag_value);
                Ok(false)
            }
        }
    }
    
    async fn get_id_tag_auth_status(&self, id_tag_value: &str) -> DomainResult<Option<String>> {
        if id_tag_value.is_empty() {
            return Ok(None);
        }

        // Look up the ID tag in database
        let tag = id_tag::Entity::find_by_id(id_tag_value)
            .one(&self.db)
            .await
            .map_err(|e| DomainError::Validation(format!("Database error: {}", e)))?;

        match tag {
            Some(t) => {
                // Get the proper OCPP auth status
                let status = t.get_auth_status();
                let status_str = match status {
                    id_tag::IdTagStatus::Accepted => "Accepted",
                    id_tag::IdTagStatus::Blocked => "Blocked",
                    id_tag::IdTagStatus::Expired => "Expired",
                    id_tag::IdTagStatus::Invalid => "Invalid",
                    id_tag::IdTagStatus::ConcurrentTx => "ConcurrentTx",
                };
                
                // Update last_used_at timestamp if accepted
                if status == id_tag::IdTagStatus::Accepted {
                    let mut active: id_tag::ActiveModel = t.into();
                    active.last_used_at = Set(Some(Utc::now()));
                    let _ = active.update(&self.db).await;
                }
                
                Ok(Some(status_str.to_string()))
            }
            None => {
                // Tag not found
                Ok(None)
            }
        }
    }

    async fn add_id_tag(&self, id_tag_value: String) -> DomainResult<()> {
        let now = Utc::now();
        
        let new_tag = id_tag::ActiveModel {
            id_tag: Set(id_tag_value),
            parent_id_tag: Set(None),
            status: Set(id_tag::IdTagStatus::Accepted),
            user_id: Set(None),
            name: Set(None),
            expiry_date: Set(None),
            max_active_transactions: Set(None),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            last_used_at: Set(None),
        };
        
        new_tag.insert(&self.db).await
            .map_err(|e| DomainError::Validation(format!("Database error: {}", e)))?;
        
        Ok(())
    }

    async fn remove_id_tag(&self, id_tag_value: &str) -> DomainResult<()> {
        id_tag::Entity::delete_by_id(id_tag_value)
            .exec(&self.db)
            .await
            .map_err(|e| DomainError::Validation(format!("Database error: {}", e)))?;
        
        Ok(())
    }

    async fn next_transaction_id(&self) -> i32 {
        // For SQLite, we rely on auto-increment
        // Get max ID + 1 as a hint
        let result = transaction::Entity::find()
            .all(&self.db)
            .await
            .map(|txs| txs.into_iter().map(|t| t.id).max().unwrap_or(0) + 1)
            .unwrap_or(1);

        result
    }

    // Tariff operations
    
    async fn get_tariff(&self, id: i32) -> DomainResult<Option<Tariff>> {
        let model = tariff::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;
        
        Ok(model.map(tariff_entity_to_domain))
    }
    
    async fn get_default_tariff(&self) -> DomainResult<Option<Tariff>> {
        let model = tariff::Entity::find()
            .filter(tariff::Column::IsDefault.eq(true))
            .filter(tariff::Column::IsActive.eq(true))
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;
        
        Ok(model.map(tariff_entity_to_domain))
    }
    
    async fn list_tariffs(&self) -> DomainResult<Vec<Tariff>> {
        let models = tariff::Entity::find()
            .order_by_asc(tariff::Column::Name)
            .all(&self.db)
            .await
            .map_err(db_error_to_domain)?;
        
        Ok(models.into_iter().map(tariff_entity_to_domain).collect())
    }
    
    async fn save_tariff(&self, t: Tariff) -> DomainResult<Tariff> {
        let now = Utc::now();
        
        let model = tariff::ActiveModel {
            id: Set(0), // Auto-increment
            name: Set(t.name),
            description: Set(t.description),
            tariff_type: Set(domain_tariff_type_to_entity(&t.tariff_type)),
            price_per_kwh: Set(t.price_per_kwh),
            price_per_minute: Set(t.price_per_minute),
            session_fee: Set(t.session_fee),
            currency: Set(t.currency),
            min_fee: Set(t.min_fee),
            max_fee: Set(t.max_fee),
            is_active: Set(t.is_active),
            is_default: Set(t.is_default),
            valid_from: Set(t.valid_from),
            valid_until: Set(t.valid_until),
            created_at: Set(now),
            updated_at: Set(now),
        };
        
        let result = model.insert(&self.db).await.map_err(db_error_to_domain)?;
        info!("Tariff saved: {} ({})", result.name, result.id);
        
        Ok(tariff_entity_to_domain(result))
    }
    
    async fn update_tariff(&self, t: Tariff) -> DomainResult<()> {
        let existing = tariff::Entity::find_by_id(t.id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;
        
        if existing.is_none() {
            return Err(DomainError::NotFound { entity: "Tariff", field: "id", value: t.id.to_string() });
        }
        
        let model = tariff::ActiveModel {
            id: Set(t.id),
            name: Set(t.name),
            description: Set(t.description),
            tariff_type: Set(domain_tariff_type_to_entity(&t.tariff_type)),
            price_per_kwh: Set(t.price_per_kwh),
            price_per_minute: Set(t.price_per_minute),
            session_fee: Set(t.session_fee),
            currency: Set(t.currency),
            min_fee: Set(t.min_fee),
            max_fee: Set(t.max_fee),
            is_active: Set(t.is_active),
            is_default: Set(t.is_default),
            valid_from: Set(t.valid_from),
            valid_until: Set(t.valid_until),
            created_at: Set(existing.unwrap().created_at),
            updated_at: Set(Utc::now()),
        };
        
        model.update(&self.db).await.map_err(db_error_to_domain)?;
        Ok(())
    }
    
    async fn delete_tariff(&self, id: i32) -> DomainResult<()> {
        let result = tariff::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(db_error_to_domain)?;
        
        if result.rows_affected == 0 {
            return Err(DomainError::NotFound { entity: "Tariff", field: "id", value: id.to_string() });
        }
        
        Ok(())
    }
    
    // Billing operations
    
    async fn update_transaction_billing(&self, billing: TransactionBilling) -> DomainResult<()> {
        let existing = transaction::Entity::find_by_id(billing.transaction_id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;
        
        let Some(tx) = existing else {
            return Err(DomainError::NotFound { entity: "Transaction", field: "id", value: billing.transaction_id.to_string() });
        };
        
        let mut model: transaction::ActiveModel = tx.into();
        model.tariff_id = Set(billing.tariff_id);
        model.energy_cost = Set(Some(billing.energy_cost));
        model.time_cost = Set(Some(billing.time_cost));
        model.session_fee = Set(Some(billing.session_fee));
        model.total_cost = Set(Some(billing.total_cost));
        model.currency = Set(Some(billing.currency));
        model.billing_status = Set(Some(billing.status.to_string()));
        
        model.update(&self.db).await.map_err(db_error_to_domain)?;
        
        info!("Transaction {} billing updated: total={}", billing.transaction_id, billing.total_cost);
        Ok(())
    }
    
    async fn get_transaction_billing(&self, transaction_id: i32) -> DomainResult<Option<TransactionBilling>> {
        let tx = transaction::Entity::find_by_id(transaction_id)
            .one(&self.db)
            .await
            .map_err(db_error_to_domain)?;
        
        let Some(tx) = tx else {
            return Ok(None);
        };
        
        // Only return billing if it has been calculated
        if tx.billing_status.is_none() || tx.total_cost.is_none() {
            return Ok(None);
        }
        
        let duration_seconds = tx.stopped_at
            .map(|stop| (stop - tx.started_at).num_seconds())
            .unwrap_or(0);
        
        Ok(Some(TransactionBilling {
            transaction_id: tx.id,
            tariff_id: tx.tariff_id,
            energy_wh: tx.energy_consumed.unwrap_or(0),
            duration_seconds,
            energy_cost: tx.energy_cost.unwrap_or(0),
            time_cost: tx.time_cost.unwrap_or(0),
            session_fee: tx.session_fee.unwrap_or(0),
            total_cost: tx.total_cost.unwrap_or(0),
            currency: tx.currency.unwrap_or_else(|| "UZS".to_string()),
            status: string_to_billing_status(&tx.billing_status.unwrap_or_default()),
        }))
    }
}
