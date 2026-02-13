//! Reservation repository interface

use async_trait::async_trait;

use super::model::Reservation;
use crate::domain::DomainResult;

#[async_trait]
pub trait ReservationRepository: Send + Sync {
    /// Save a new reservation
    async fn save(&self, reservation: Reservation) -> DomainResult<()>;

    /// Find reservation by ID
    async fn find_by_id(&self, id: i32) -> DomainResult<Option<Reservation>>;

    /// Update an existing reservation
    async fn update(&self, reservation: Reservation) -> DomainResult<()>;

    /// Find all active reservations for a charge point
    async fn find_active_for_charge_point(
        &self,
        charge_point_id: &str,
    ) -> DomainResult<Vec<Reservation>>;

    /// Find active reservation for a specific connector
    async fn find_active_for_connector(
        &self,
        charge_point_id: &str,
        connector_id: i32,
    ) -> DomainResult<Option<Reservation>>;

    /// Find all reservations (any status)
    async fn find_all(&self) -> DomainResult<Vec<Reservation>>;

    /// Find all expired active reservations (expiry_date < now, status = Accepted)
    async fn find_expired(&self) -> DomainResult<Vec<Reservation>>;

    /// Cancel a reservation by ID (set status = Cancelled)
    async fn cancel(&self, id: i32) -> DomainResult<()>;

    /// Generate next reservation ID
    async fn next_id(&self) -> i32;
}
