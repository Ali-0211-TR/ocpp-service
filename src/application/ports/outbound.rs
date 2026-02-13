//! Outbound ports — interfaces for sending OCPP commands to charge points
//!
//! [`OcppOutboundPort`] is the architectural contract that decouples domain /
//! application services from the concrete command transport layer.
//!
//! The single production implementation lives in
//! [`CommandDispatcher`](crate::application::charging::commands::dispatcher::CommandDispatcher),
//! which resolves the charge-point's negotiated OCPP version and delegates
//! to version-specific serialisers (`v16::*`, `v201::*`).

use async_trait::async_trait;

use crate::application::charging::commands::dispatcher::ClearChargingProfileCriteria;
use crate::application::charging::commands::dispatcher::GetDiagnosticsResult;
use crate::application::charging::commands::{
    Availability, CommandError, CompositeScheduleResult, ConfigurationResult, DataTransferResult,
    LocalAuthEntry, ResetKind, TriggerType,
};
use crate::application::charging::commands::dispatcher::{
    GetVariablesResult, SetVariablesResult,
};

// ── Generic status (version-agnostic) ──────────────────────────

/// Version-agnostic command response status.
///
/// Maps to `ParsedGenericStatus` in OCPP 1.6, `GenericStatusEnum` in 2.0.1, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericStatus {
    Accepted,
    Rejected,
    Scheduled,
    NotSupported,
    Faulted,
    Unknown(String),
}

impl GenericStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Accepted" => Self::Accepted,
            "Rejected" => Self::Rejected,
            "Scheduled" => Self::Scheduled,
            "NotSupported" => Self::NotSupported,
            "Faulted" => Self::Faulted,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }
}

impl std::fmt::Display for GenericStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accepted => write!(f, "Accepted"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Scheduled => write!(f, "Scheduled"),
            Self::NotSupported => write!(f, "NotSupported"),
            Self::Faulted => write!(f, "Faulted"),
            Self::Unknown(s) => write!(f, "{}", s),
        }
    }
}

// ── OcppOutboundPort ───────────────────────────────────────────

/// Port for sending commands from the central system to a charge point.
///
/// The trait is version-agnostic: implementations (like [`CommandDispatcher`])
/// resolve the charge-point's protocol version internally and serialise
/// the correct OCPP wire format.
///
/// All methods return `Result<String, CommandError>` where the `String` is
/// the response status (e.g. `"Accepted"`, `"Rejected"`), except for a
/// few commands that return richer result types.
#[async_trait]
pub trait OcppOutboundPort: Send + Sync {
    // ── Session / Transaction ──────────────────────────────────

    /// Send a RemoteStartTransaction command.
    async fn remote_start_transaction(
        &self,
        charge_point_id: &str,
        id_tag: &str,
        connector_id: Option<u32>,
    ) -> Result<String, CommandError>;

    /// Send a RemoteStopTransaction command.
    async fn remote_stop_transaction(
        &self,
        charge_point_id: &str,
        transaction_id: i32,
    ) -> Result<String, CommandError>;

    // ── Station management ─────────────────────────────────────

    /// Send a Reset command.
    async fn reset(
        &self,
        charge_point_id: &str,
        reset_type: ResetKind,
    ) -> Result<String, CommandError>;

    /// Send an UnlockConnector command.
    async fn unlock_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> Result<String, CommandError>;

    /// Send a ChangeAvailability command.
    async fn change_availability(
        &self,
        charge_point_id: &str,
        connector_id: u32,
        availability: Availability,
    ) -> Result<String, CommandError>;

    /// Send a ClearCache command.
    async fn clear_cache(&self, charge_point_id: &str) -> Result<String, CommandError>;

    /// Send a TriggerMessage command.
    async fn trigger_message(
        &self,
        charge_point_id: &str,
        requested_message: TriggerType,
        connector_id: Option<u32>,
    ) -> Result<String, CommandError>;

    // ── Configuration ──────────────────────────────────────────

    /// GetConfiguration — v1.6 only. Returns `UnsupportedVersion` for v2.0.1.
    async fn get_configuration(
        &self,
        charge_point_id: &str,
        keys: Option<Vec<String>>,
    ) -> Result<ConfigurationResult, CommandError>;

    /// ChangeConfiguration — v1.6 only. Returns `UnsupportedVersion` for v2.0.1.
    async fn change_configuration(
        &self,
        charge_point_id: &str,
        key: String,
        value: String,
    ) -> Result<String, CommandError>;

    /// GetVariables — v2.0.1 only. Returns `UnsupportedVersion` for v1.6.
    ///
    /// `variables` is a list of `(component_name, variable_name)` pairs.
    async fn get_variables(
        &self,
        charge_point_id: &str,
        variables: Vec<(String, String)>,
    ) -> Result<GetVariablesResult, CommandError>;

    /// SetVariables — v2.0.1 only. Returns `UnsupportedVersion` for v1.6.
    ///
    /// `variables` is a list of `(component_name, variable_name, value)` tuples.
    async fn set_variables(
        &self,
        charge_point_id: &str,
        variables: Vec<(String, String, String)>,
    ) -> Result<SetVariablesResult, CommandError>;

    // ── Charging profiles (v1.6 + v2.0.1) ───────────────────────

    /// ClearChargingProfile — removes one or more charging profiles.
    async fn clear_charging_profile(
        &self,
        charge_point_id: &str,
        criteria: ClearChargingProfileCriteria,
    ) -> Result<String, CommandError>;

    /// SetChargingProfile — sends a charging profile to the charge point.
    ///
    /// Accepts the profile as a raw `serde_json::Value` so callers don't need
    /// to depend on `rust_ocpp` types directly. Version-specific deserialization
    /// happens inside the dispatcher.
    async fn set_charging_profile(
        &self,
        charge_point_id: &str,
        evse_id: i32,
        charging_profile_json: serde_json::Value,
    ) -> Result<String, CommandError>;

    /// GetCompositeSchedule — request the composite charging schedule.
    ///
    /// `connector_or_evse_id` maps to `connector_id` (v1.6) or `evse_id` (v2.0.1).
    async fn get_composite_schedule(
        &self,
        charge_point_id: &str,
        connector_or_evse_id: i32,
        duration: i32,
        charging_rate_unit: Option<&str>,
    ) -> Result<CompositeScheduleResult, CommandError>;

    // ── Data transfer ──────────────────────────────────────────

    /// Send a DataTransfer command.
    async fn data_transfer(
        &self,
        charge_point_id: &str,
        vendor_id: String,
        message_id: Option<String>,
        data: Option<String>,
    ) -> Result<DataTransferResult, CommandError>;

    // ── Local auth list ────────────────────────────────────────

    /// Get the local authorization list version.
    async fn get_local_list_version(
        &self,
        charge_point_id: &str,
    ) -> Result<i32, CommandError>;

    /// Send (full or differential) local authorization list update.
    ///
    /// `update_type`: `"Full"` or `"Differential"`.
    async fn send_local_list(
        &self,
        charge_point_id: &str,
        list_version: i32,
        update_type: &str,
        entries: Option<Vec<LocalAuthEntry>>,
    ) -> Result<String, CommandError>;

    // ── Reservations (v1.6 + v2.0.1) ──────────────────────────

    /// ReserveNow — reserve a connector/EVSE for an ID tag.
    ///
    /// `connector_id` = 0 means "any connector" (v1.6) or no `evse_id` (v2.0.1).
    async fn reserve_now(
        &self,
        charge_point_id: &str,
        reservation_id: i32,
        connector_id: i32,
        id_tag: &str,
        parent_id_tag: Option<&str>,
        expiry_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<String, CommandError>;

    /// CancelReservation — cancel an existing reservation.
    async fn cancel_reservation(
        &self,
        charge_point_id: &str,
        reservation_id: i32,
    ) -> Result<String, CommandError>;

    // ─── Firmware Management ───────────────────────────────────────────

    /// UpdateFirmware — instruct a charge point to download and install firmware.
    async fn update_firmware(
        &self,
        charge_point_id: &str,
        location: &str,
        retrieve_date: chrono::DateTime<chrono::Utc>,
        retries: Option<i32>,
        retry_interval: Option<i32>,
    ) -> Result<String, CommandError>;

    /// GetDiagnostics (v1.6) / GetLog (v2.0.1) — request diagnostic upload.
    async fn get_diagnostics(
        &self,
        charge_point_id: &str,
        location: &str,
        retries: Option<i32>,
        retry_interval: Option<i32>,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        stop_time: Option<chrono::DateTime<chrono::Utc>>,
        log_type: Option<&str>,
    ) -> Result<GetDiagnosticsResult, CommandError>;
}
