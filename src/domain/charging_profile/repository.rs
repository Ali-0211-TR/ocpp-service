//! ChargingProfile repository interface

use async_trait::async_trait;

use super::model::ChargingProfile;
use crate::domain::DomainResult;

#[async_trait]
pub trait ChargingProfileRepository: Send + Sync {
    /// Save a new charging profile record.
    async fn save(&self, profile: ChargingProfile) -> DomainResult<ChargingProfile>;

    /// Find all active profiles for a charge point.
    async fn find_active_for_charge_point(
        &self,
        charge_point_id: &str,
    ) -> DomainResult<Vec<ChargingProfile>>;

    /// Find all profiles (active + inactive) for a charge point.
    async fn find_all_for_charge_point(
        &self,
        charge_point_id: &str,
    ) -> DomainResult<Vec<ChargingProfile>>;

    /// Deactivate a specific profile by its OCPP profile_id on a charge point.
    async fn deactivate_by_profile_id(
        &self,
        charge_point_id: &str,
        profile_id: i32,
    ) -> DomainResult<u64>;

    /// Deactivate profiles matching criteria (used by ClearChargingProfile).
    ///
    /// Criteria fields are optional — if `None`, that field is not filtered.
    /// Returns the number of profiles deactivated.
    async fn deactivate_by_criteria(
        &self,
        charge_point_id: &str,
        evse_id: Option<i32>,
        purpose: Option<&str>,
        stack_level: Option<i32>,
    ) -> DomainResult<u64>;

    /// Deactivate ALL active profiles for a charge point.
    async fn deactivate_all(&self, charge_point_id: &str) -> DomainResult<u64>;
}
