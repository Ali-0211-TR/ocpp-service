//! SeaORM implementations of TariffRepository and BillingRepository

use async_trait::async_trait;
use chrono::Utc;
use log::info;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::domain::tariff::{
    BillingRepository, BillingStatus, Tariff, TariffRepository, TariffType, TransactionBilling,
};
use crate::domain::{DomainError, DomainResult};
use crate::infrastructure::database::entities::{tariff, transaction};

// ── Conversion helpers ──────────────────────────────────────────

fn db_err(e: sea_orm::DbErr) -> DomainError {
    DomainError::Validation(format!("Database error: {}", e))
}

fn entity_to_domain(t: tariff::Model) -> Tariff {
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

fn type_to_entity(t: &TariffType) -> tariff::TariffType {
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

// ── SeaOrmTariffRepository ──────────────────────────────────────

pub struct SeaOrmTariffRepository {
    db: DatabaseConnection,
}

impl SeaOrmTariffRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TariffRepository for SeaOrmTariffRepository {
    async fn find_by_id(&self, id: i32) -> DomainResult<Option<Tariff>> {
        let model = tariff::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(model.map(entity_to_domain))
    }

    async fn find_default(&self) -> DomainResult<Option<Tariff>> {
        let model = tariff::Entity::find()
            .filter(tariff::Column::IsDefault.eq(true))
            .filter(tariff::Column::IsActive.eq(true))
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(model.map(entity_to_domain))
    }

    async fn find_all(&self) -> DomainResult<Vec<Tariff>> {
        let models = tariff::Entity::find()
            .order_by_asc(tariff::Column::Name)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        Ok(models.into_iter().map(entity_to_domain).collect())
    }

    async fn save(&self, t: Tariff) -> DomainResult<Tariff> {
        let now = Utc::now();
        let model = tariff::ActiveModel {
            id: Set(0),
            name: Set(t.name),
            description: Set(t.description),
            tariff_type: Set(type_to_entity(&t.tariff_type)),
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
        let result = model.insert(&self.db).await.map_err(db_err)?;
        info!("Tariff saved: {} ({})", result.name, result.id);
        Ok(entity_to_domain(result))
    }

    async fn update(&self, t: Tariff) -> DomainResult<()> {
        let existing = tariff::Entity::find_by_id(t.id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        let Some(existing) = existing else {
            return Err(DomainError::NotFound {
                entity: "Tariff",
                field: "id",
                value: t.id.to_string(),
            });
        };

        let model = tariff::ActiveModel {
            id: Set(t.id),
            name: Set(t.name),
            description: Set(t.description),
            tariff_type: Set(type_to_entity(&t.tariff_type)),
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
            created_at: Set(existing.created_at),
            updated_at: Set(Utc::now()),
        };
        model.update(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: i32) -> DomainResult<()> {
        let result = tariff::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(db_err)?;
        if result.rows_affected == 0 {
            return Err(DomainError::NotFound {
                entity: "Tariff",
                field: "id",
                value: id.to_string(),
            });
        }
        Ok(())
    }
}

// ── SeaOrmBillingRepository ─────────────────────────────────────

pub struct SeaOrmBillingRepository {
    db: DatabaseConnection,
}

impl SeaOrmBillingRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BillingRepository for SeaOrmBillingRepository {
    async fn update_billing(&self, billing: TransactionBilling) -> DomainResult<()> {
        let existing = transaction::Entity::find_by_id(billing.transaction_id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        let Some(tx) = existing else {
            return Err(DomainError::NotFound {
                entity: "Transaction",
                field: "id",
                value: billing.transaction_id.to_string(),
            });
        };

        let mut model: transaction::ActiveModel = tx.into();
        model.tariff_id = Set(billing.tariff_id);
        model.energy_cost = Set(Some(billing.energy_cost));
        model.time_cost = Set(Some(billing.time_cost));
        model.session_fee = Set(Some(billing.session_fee));
        model.total_cost = Set(Some(billing.total_cost));
        model.currency = Set(Some(billing.currency));
        model.billing_status = Set(Some(billing.status.to_string()));
        model.update(&self.db).await.map_err(db_err)?;

        info!(
            "Transaction {} billing updated: total={}",
            billing.transaction_id, billing.total_cost
        );
        Ok(())
    }

    async fn get_billing(&self, transaction_id: i32) -> DomainResult<Option<TransactionBilling>> {
        let tx = transaction::Entity::find_by_id(transaction_id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        let Some(tx) = tx else {
            return Ok(None);
        };

        if tx.billing_status.is_none() || tx.total_cost.is_none() {
            return Ok(None);
        }

        let duration_seconds = tx
            .stopped_at
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
