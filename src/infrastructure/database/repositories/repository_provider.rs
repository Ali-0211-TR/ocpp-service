//! SeaORM implementation of RepositoryProvider

use sea_orm::DatabaseConnection;

use crate::domain::charge_point::ChargePointRepository;
use crate::domain::id_tag::IdTagRepository;
use crate::domain::repositories::RepositoryProvider;
use crate::domain::tariff::{BillingRepository, TariffRepository};
use crate::domain::transaction::TransactionRepository;

use super::charge_point_repository::SeaOrmChargePointRepository;
use super::id_tag_repository::SeaOrmIdTagRepository;
use super::tariff_repository::{SeaOrmBillingRepository, SeaOrmTariffRepository};
use super::transaction_repository::SeaOrmTransactionRepository;

/// Unified repository provider backed by SeaORM.
///
/// Holds one connection pool and exposes per-aggregate repository accessors.
///
/// ```ignore
/// let repos = SeaOrmRepositoryProvider::new(db.clone());
/// let cp = repos.charge_points().find_by_id("CP001").await?;
/// let tx = repos.transactions().find_active_for_connector("CP001", 1).await?;
/// ```
pub struct SeaOrmRepositoryProvider {
    charge_points: SeaOrmChargePointRepository,
    transactions: SeaOrmTransactionRepository,
    id_tags: SeaOrmIdTagRepository,
    tariffs: SeaOrmTariffRepository,
    billing: SeaOrmBillingRepository,
}

impl SeaOrmRepositoryProvider {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            charge_points: SeaOrmChargePointRepository::new(db.clone()),
            transactions: SeaOrmTransactionRepository::new(db.clone()),
            id_tags: SeaOrmIdTagRepository::new(db.clone()),
            tariffs: SeaOrmTariffRepository::new(db.clone()),
            billing: SeaOrmBillingRepository::new(db),
        }
    }
}

impl RepositoryProvider for SeaOrmRepositoryProvider {
    fn charge_points(&self) -> &dyn ChargePointRepository {
        &self.charge_points
    }

    fn transactions(&self) -> &dyn TransactionRepository {
        &self.transactions
    }

    fn id_tags(&self) -> &dyn IdTagRepository {
        &self.id_tags
    }

    fn tariffs(&self) -> &dyn TariffRepository {
        &self.tariffs
    }

    fn billing(&self) -> &dyn BillingRepository {
        &self.billing
    }
}
