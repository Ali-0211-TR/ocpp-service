//! Tariff & billing repository interfaces

use async_trait::async_trait;

use super::model::{Tariff, TransactionBilling};
use crate::domain::DomainResult;

#[async_trait]
pub trait TariffRepository: Send + Sync {
    async fn find_by_id(&self, id: i32) -> DomainResult<Option<Tariff>>;
    async fn find_default(&self) -> DomainResult<Option<Tariff>>;
    async fn find_all(&self) -> DomainResult<Vec<Tariff>>;
    async fn save(&self, tariff: Tariff) -> DomainResult<Tariff>;
    async fn update(&self, tariff: Tariff) -> DomainResult<()>;
    async fn delete(&self, id: i32) -> DomainResult<()>;
}

#[async_trait]
pub trait BillingRepository: Send + Sync {
    async fn update_billing(&self, billing: TransactionBilling) -> DomainResult<()>;
    async fn get_billing(&self, transaction_id: i32) -> DomainResult<Option<TransactionBilling>>;
}
