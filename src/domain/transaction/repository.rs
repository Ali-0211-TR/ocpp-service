//! Transaction repository interface

use async_trait::async_trait;

use super::model::Transaction;
use crate::domain::DomainResult;

#[async_trait]
pub trait TransactionRepository: Send + Sync {
    async fn save(&self, transaction: Transaction) -> DomainResult<()>;
    async fn find_by_id(&self, id: i32) -> DomainResult<Option<Transaction>>;
    async fn update(&self, transaction: Transaction) -> DomainResult<()>;
    async fn find_active_for_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> DomainResult<Option<Transaction>>;
    async fn find_by_charge_point(&self, charge_point_id: &str) -> DomainResult<Vec<Transaction>>;
    async fn find_all(&self) -> DomainResult<Vec<Transaction>>;
    async fn update_meter_data(
        &self,
        transaction_id: i32,
        meter_value: Option<i32>,
        power_w: Option<f64>,
        soc: Option<i32>,
    ) -> DomainResult<()>;
    async fn next_id(&self) -> i32;
}
