//! Tariff aggregate
//!
//! Contains the Tariff entity, billing types, cost calculation logic,
//! and repository interfaces.

pub mod model;
pub mod repository;

pub use model::{BillingStatus, CostBreakdown, Tariff, TariffType, TransactionBilling};
pub use repository::{BillingRepository, TariffRepository};
