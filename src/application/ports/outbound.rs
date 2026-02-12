//! Outbound ports — interfaces for sending OCPP commands to charge points
//!
//! Each OCPP version provides an adapter that implements `OcppOutboundPort`
//! to handle central-system → charge-point commands.
//!
//! **Phase 2**: This trait will be fully implemented with version-specific
//! adapters for constructing and sending commands. For now it serves as
//! the architectural contract.

use async_trait::async_trait;

use crate::application::CommandError;

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
/// Version-specific adapters implement this trait, translating generic
/// command requests into the appropriate OCPP version's wire format.
///
/// **Phase 2**: Methods below will be expanded to cover all CS→CP operations.
#[async_trait]
pub trait OcppOutboundPort: Send + Sync {
    /// Send a RemoteStartTransaction command.
    async fn remote_start_transaction(
        &self,
        charge_point_id: &str,
        id_tag: &str,
        connector_id: Option<u32>,
    ) -> Result<GenericStatus, CommandError>;

    /// Send a RemoteStopTransaction command.
    async fn remote_stop_transaction(
        &self,
        charge_point_id: &str,
        transaction_id: i32,
    ) -> Result<GenericStatus, CommandError>;

    /// Send a Reset command.
    async fn reset(&self, charge_point_id: &str, soft: bool)
        -> Result<GenericStatus, CommandError>;

    /// Send an UnlockConnector command.
    async fn unlock_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> Result<GenericStatus, CommandError>;
}
