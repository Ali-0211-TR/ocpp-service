//! Charge Point repository interface

use async_trait::async_trait;

use super::model::{ChargePoint, ChargePointStatus};
use crate::domain::DomainResult;

#[async_trait]
pub trait ChargePointRepository: Send + Sync {
    async fn save(&self, charge_point: ChargePoint) -> DomainResult<()>;
    async fn find_by_id(&self, id: &str) -> DomainResult<Option<ChargePoint>>;
    async fn find_all(&self) -> DomainResult<Vec<ChargePoint>>;
    async fn update(&self, charge_point: ChargePoint) -> DomainResult<()>;
    async fn update_status(&self, id: &str, status: ChargePointStatus) -> DomainResult<()>;
    async fn delete(&self, id: &str) -> DomainResult<()>;
}
