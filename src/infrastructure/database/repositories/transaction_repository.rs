//! SeaORM implementation of TransactionRepository

use async_trait::async_trait;
use chrono::Utc;
use log::debug;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::domain::transaction::{ChargingLimitType, Transaction, TransactionRepository, TransactionStatus};
use crate::domain::{DomainError, DomainResult};
use crate::infrastructure::database::entities::transaction;

pub struct SeaOrmTransactionRepository {
    db: DatabaseConnection,
}

impl SeaOrmTransactionRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Conversion helpers ──────────────────────────────────────────

fn status_to_string(status: &TransactionStatus) -> String {
    match status {
        TransactionStatus::Active => "Active",
        TransactionStatus::Completed => "Completed",
        TransactionStatus::Failed => "Failed",
    }
    .to_string()
}

fn string_to_status(s: &str) -> TransactionStatus {
    match s {
        "Active" => TransactionStatus::Active,
        "Completed" => TransactionStatus::Completed,
        "Failed" => TransactionStatus::Failed,
        _ => TransactionStatus::Failed,
    }
}

fn model_to_domain(t: transaction::Model) -> Transaction {
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
        status: string_to_status(&t.status),
        last_meter_value: t.last_meter_value,
        current_power_w: t.current_power_w,
        current_soc: t.current_soc,
        last_meter_update: t.last_meter_update,
        limit_type: t.limit_type.as_deref().and_then(ChargingLimitType::from_str),
        limit_value: t.limit_value,
        external_order_id: t.external_order_id,
    }
}

fn db_err(e: sea_orm::DbErr) -> DomainError {
    DomainError::Validation(format!("Database error: {}", e))
}

// ── TransactionRepository impl ──────────────────────────────────

#[async_trait]
impl TransactionRepository for SeaOrmTransactionRepository {
    async fn save(&self, tx: Transaction) -> DomainResult<()> {
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
            status: Set(status_to_string(&tx.status)),
            tariff_id: Set(None),
            total_cost: Set(None),
            currency: Set(None),
            energy_cost: Set(None),
            time_cost: Set(None),
            session_fee: Set(None),
            billing_status: Set(Some("Pending".to_string())),
            last_meter_value: Set(tx.last_meter_value),
            current_power_w: Set(tx.current_power_w),
            current_soc: Set(tx.current_soc),
            last_meter_update: Set(tx.last_meter_update),
            limit_type: Set(tx.limit_type.as_ref().map(|lt| lt.as_str().to_string())),
            limit_value: Set(tx.limit_value),
            external_order_id: Set(tx.external_order_id),
        };
        model.insert(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn find_by_id(&self, id: i32) -> DomainResult<Option<Transaction>> {
        let model = transaction::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(model.map(model_to_domain))
    }

    async fn update(&self, tx: Transaction) -> DomainResult<()> {
        debug!("Updating transaction: {}", tx.id);

        let existing = transaction::Entity::find_by_id(tx.id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        let Some(existing) = existing else {
            return Err(DomainError::NotFound {
                entity: "Transaction",
                field: "id",
                value: tx.id.to_string(),
            });
        };

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
            status: Set(status_to_string(&tx.status)),
            tariff_id: Set(existing.tariff_id),
            total_cost: Set(existing.total_cost),
            currency: Set(existing.currency),
            energy_cost: Set(existing.energy_cost),
            time_cost: Set(existing.time_cost),
            session_fee: Set(existing.session_fee),
            billing_status: Set(existing.billing_status),
            last_meter_value: Set(tx.last_meter_value),
            current_power_w: Set(tx.current_power_w),
            current_soc: Set(tx.current_soc),
            last_meter_update: Set(tx.last_meter_update),
            limit_type: Set(tx.limit_type.as_ref().map(|lt| lt.as_str().to_string())),
            limit_value: Set(tx.limit_value),
            external_order_id: Set(tx.external_order_id),
        };
        model.update(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn find_active_for_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> DomainResult<Option<Transaction>> {
        let model = transaction::Entity::find()
            .filter(transaction::Column::ChargePointId.eq(charge_point_id))
            .filter(transaction::Column::ConnectorId.eq(connector_id as i32))
            .filter(transaction::Column::Status.eq("Active"))
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(model.map(model_to_domain))
    }

    async fn find_by_charge_point(&self, charge_point_id: &str) -> DomainResult<Vec<Transaction>> {
        let models = transaction::Entity::find()
            .filter(transaction::Column::ChargePointId.eq(charge_point_id))
            .all(&self.db)
            .await
            .map_err(db_err)?;
        Ok(models.into_iter().map(model_to_domain).collect())
    }

    async fn find_all(&self) -> DomainResult<Vec<Transaction>> {
        let models = transaction::Entity::find()
            .order_by_desc(transaction::Column::Id)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        Ok(models.into_iter().map(model_to_domain).collect())
    }

    async fn update_meter_data(
        &self,
        transaction_id: i32,
        meter_value: Option<i32>,
        power_w: Option<f64>,
        soc: Option<i32>,
    ) -> DomainResult<()> {
        debug!(
            "Updating meter data for transaction {}: meter={:?}, power={:?}, soc={:?}",
            transaction_id, meter_value, power_w, soc
        );

        let existing = transaction::Entity::find_by_id(transaction_id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        let Some(existing) = existing else {
            return Err(DomainError::NotFound {
                entity: "Transaction",
                field: "id",
                value: transaction_id.to_string(),
            });
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
        active.update(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn next_id(&self) -> i32 {
        transaction::Entity::find()
            .all(&self.db)
            .await
            .map(|txs| txs.into_iter().map(|t| t.id).max().unwrap_or(0) + 1)
            .unwrap_or(1)
    }
}
