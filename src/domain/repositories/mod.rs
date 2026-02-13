//! Repository traits for the domain layer
//!
//! Contains:
//! - `RepositoryProvider` — unified access to all per-aggregate repositories
//! - `Storage` — legacy monolithic trait (kept for backward compatibility during migration)
//! - `DomainResult` — standard result type for domain operations

use super::charge_point::ChargePointRepository;
use super::id_tag::IdTagRepository;
use super::reservation::ReservationRepository;
use super::tariff::{BillingRepository, TariffRepository};
use super::transaction::TransactionRepository;
use crate::shared::errors::DomainError;

/// Result type for domain operations
pub type DomainResult<T> = Result<T, DomainError>;

// ── RepositoryProvider ──────────────────────────────────────────

/// Provides access to all domain repositories.
///
/// Replaces the monolithic `Storage` trait. Consumers request
/// only the repository they need:
///
/// ```ignore
/// async fn handle(repos: &dyn RepositoryProvider) {
///     let cp = repos.charge_points().find_by_id("CP001").await?;
///     let tx = repos.transactions().find_active_for_connector("CP001", 1).await?;
/// }
/// ```
pub trait RepositoryProvider: Send + Sync {
    fn charge_points(&self) -> &dyn ChargePointRepository;
    fn transactions(&self) -> &dyn TransactionRepository;
    fn id_tags(&self) -> &dyn IdTagRepository;
    fn tariffs(&self) -> &dyn TariffRepository;
    fn billing(&self) -> &dyn BillingRepository;
    fn reservations(&self) -> &dyn ReservationRepository;
}

// ── Legacy Storage trait removed ────────────────────────────────
//
// All consumers have been migrated to RepositoryProvider.
// See: SeaOrmRepositoryProvider in infrastructure/database/repositories/
