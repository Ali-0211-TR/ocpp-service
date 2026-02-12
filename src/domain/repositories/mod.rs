//! Repository traits for the domain layer
//!
//! The `Storage` trait spans multiple aggregates and lives here.
//! Per-aggregate repository interfaces live in their respective modules
//! (e.g. `domain::user::UserRepositoryInterface`).

use async_trait::async_trait;

use super::charge_point::{ChargePoint, ChargePointStatus};
use super::tariff::{Tariff, TransactionBilling};
use super::transaction::Transaction;
use crate::shared::errors::DomainError;

/// Result type for domain operations
pub type DomainResult<T> = Result<T, DomainError>;

/// Storage trait for persistence operations
///
/// This is the main repository interface used by application services.
/// Split into logical sections for charge points, transactions, ID tags,
/// tariffs, and billing.
#[async_trait]
pub trait Storage: Send + Sync {
    // ── Charge Point operations ─────────────────────────────────
    async fn save_charge_point(&self, charge_point: ChargePoint) -> DomainResult<()>;
    async fn get_charge_point(&self, id: &str) -> DomainResult<Option<ChargePoint>>;
    async fn update_charge_point(&self, charge_point: ChargePoint) -> DomainResult<()>;
    async fn delete_charge_point(&self, id: &str) -> DomainResult<()>;
    async fn list_charge_points(&self) -> DomainResult<Vec<ChargePoint>>;
    async fn update_charge_point_status(
        &self,
        id: &str,
        status: ChargePointStatus,
    ) -> DomainResult<()>;

    // ── Transaction operations ──────────────────────────────────
    async fn save_transaction(&self, transaction: Transaction) -> DomainResult<()>;
    async fn get_transaction(&self, id: i32) -> DomainResult<Option<Transaction>>;
    async fn update_transaction(&self, transaction: Transaction) -> DomainResult<()>;
    async fn get_active_transaction_for_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> DomainResult<Option<Transaction>>;
    async fn list_transactions_for_charge_point(
        &self,
        charge_point_id: &str,
    ) -> DomainResult<Vec<Transaction>>;
    async fn list_all_transactions(&self) -> DomainResult<Vec<Transaction>>;
    async fn update_transaction_meter_data(
        &self,
        transaction_id: i32,
        meter_value: Option<i32>,
        power_w: Option<f64>,
        soc: Option<i32>,
    ) -> DomainResult<()>;

    // ── ID Tag operations (authorization) ───────────────────────
    async fn is_id_tag_valid(&self, id_tag: &str) -> DomainResult<bool>;
    async fn get_id_tag_auth_status(&self, id_tag: &str) -> DomainResult<Option<String>>;
    async fn add_id_tag(&self, id_tag: String) -> DomainResult<()>;
    async fn remove_id_tag(&self, id_tag: &str) -> DomainResult<()>;

    // ── Tariff operations ───────────────────────────────────────
    async fn get_tariff(&self, id: i32) -> DomainResult<Option<Tariff>>;
    async fn get_default_tariff(&self) -> DomainResult<Option<Tariff>>;
    async fn list_tariffs(&self) -> DomainResult<Vec<Tariff>>;
    async fn save_tariff(&self, tariff: Tariff) -> DomainResult<Tariff>;
    async fn update_tariff(&self, tariff: Tariff) -> DomainResult<()>;
    async fn delete_tariff(&self, id: i32) -> DomainResult<()>;

    // ── Billing operations ──────────────────────────────────────
    async fn update_transaction_billing(&self, billing: TransactionBilling) -> DomainResult<()>;
    async fn get_transaction_billing(
        &self,
        transaction_id: i32,
    ) -> DomainResult<Option<TransactionBilling>>;

    // ── Utility ─────────────────────────────────────────────────
    async fn next_transaction_id(&self) -> i32;
}
