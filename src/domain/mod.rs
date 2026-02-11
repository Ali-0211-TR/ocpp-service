pub mod models;
pub mod repositories;
pub mod services;

// Re-export commonly used types
pub use models::charge_point::{ChargePoint, ChargePointStatus, Connector, ConnectorStatus};
pub use models::id_tag::{IdTag, IdTagStatus};
pub use models::ocpp_version::OcppVersion;
pub use models::tariff::{BillingStatus, CostBreakdown, Tariff, TariffType, TransactionBilling};
pub use models::transaction::{ChargingLimitType, Transaction, TransactionStatus};
pub use models::user::{User, UserRole};
pub use models::api_key::ApiKey;
pub use repositories::{DomainResult, Storage};

// Re-export DomainError from support for convenience
pub use crate::support::errors::DomainError;