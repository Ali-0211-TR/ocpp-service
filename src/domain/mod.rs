//! Domain layer — aggregates, value objects, ports, events
//!
//! Organized by aggregate (DDD-style): each subdirectory contains
//! the entity, its DTOs, and repository interface.

// ── Aggregates ──────────────────────────────────────────────────
pub mod charge_point;
pub mod id_tag;
pub mod ocpp;
pub mod reservation;
pub mod tariff;
pub mod transaction;
pub mod user;

// ── Cross-cutting domain concerns ──────────────────────────────
pub mod events;
pub mod ports;
pub mod repositories; // Storage trait (spans multiple aggregates)

// ── Re-exports for convenience (façade pattern) ────────────────

// User aggregate
pub use user::{
    CreateUserDto, GetUserDto, UpdateUserDto, User, UserChangePasswordDto,
    UserRepositoryInterface, UserRole,
};

// ChargePoint aggregate
pub use charge_point::{
    ChargePoint, ChargePointRepository, ChargePointStatus, Connector, ConnectorStatus,
};

// Transaction aggregate
pub use transaction::{ChargingLimitType, Transaction, TransactionRepository, TransactionStatus};

// Tariff aggregate
pub use tariff::{
    BillingRepository, BillingStatus, CostBreakdown, Tariff, TariffRepository, TariffType,
    TransactionBilling,
};

// IdTag aggregate
pub use id_tag::{IdTag, IdTagRepository, IdTagStatus};

// Reservation aggregate
pub use reservation::{Reservation, ReservationRepository, ReservationStatus};

// OCPP shared types
pub use ocpp::{ApiKey, OcppVersion};

// Storage & results
pub use repositories::{DomainResult, RepositoryProvider};

// Error
pub use crate::shared::errors::DomainError;
