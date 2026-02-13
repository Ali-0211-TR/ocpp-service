//! Command sender for Central System to Charge Point communication
//!
//! ## Architecture
//!
//! ```text
//! HTTP Handler ──► CommandDispatcher ──► v16::* / v201::*
//!                        │                     │
//!                  resolve version       build typed request
//!                  from SessionRegistry  call CommandSender
//!                                              │
//!                                     ─────────┘
//!                                    CommandSender (version-agnostic transport)
//! ```
//!
//! - [`CommandSender`] — low-level transport: sends raw JSON `[2, id, action, payload]`
//!   frames and correlates responses via `DashMap<PendingRequest>`.
//! - [`CommandDispatcher`] — version-aware facade: resolves the charge-point's
//!   OCPP version from [`SessionRegistry`] and delegates to `v16::*` or `v201::*`.
//! - `v16` / `v201` — per-version modules with concrete `rust_ocpp` types.

pub mod dispatcher;
pub mod v16;
pub mod v201;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{info, warn};

use super::session::SharedSessionRegistry;
use crate::shared::ocpp_frame::OcppFrame;

// ── Common types used by both v16 and v201 implementations ─────────

/// Availability state for ChangeAvailability command (version-agnostic).
#[derive(Debug, Clone, Copy)]
pub enum Availability {
    Operative,
    Inoperative,
}

/// Reset kind for Reset command (version-agnostic).
///
/// Maps to: v1.6 `Hard`/`Soft`, v2.0.1 `Immediate`/`OnIdle`.
#[derive(Debug, Clone, Copy)]
pub enum ResetKind {
    Soft,
    Hard,
}

/// Trigger message type (version-agnostic).
#[derive(Debug, Clone, Copy)]
pub enum TriggerType {
    BootNotification,
    DiagnosticsStatusNotification,
    FirmwareStatusNotification,
    Heartbeat,
    MeterValues,
    StatusNotification,
}

/// Result of a DataTransfer command.
#[derive(Debug)]
pub struct DataTransferResult {
    pub status: String,
    pub data: Option<String>,
}

/// A configuration key-value pair returned by GetConfiguration (v1.6).
#[derive(Debug, Clone)]
pub struct KeyValue {
    pub key: String,
    pub readonly: bool,
    pub value: Option<String>,
}

/// GetConfiguration result (v1.6).
#[derive(Debug)]
pub struct ConfigurationResult {
    pub configuration_key: Vec<KeyValue>,
    pub unknown_key: Vec<String>,
}

/// A single authorization entry for the local list (version-agnostic).
///
/// Used by `SendLocalList` across both v1.6 and v2.0.1.
#[derive(Debug, Clone)]
pub struct LocalAuthEntry {
    pub id_tag: String,
    /// Authorization status: "Accepted", "Blocked", "Expired", "Invalid", etc.
    pub status: Option<String>,
    /// ISO 8601 expiry date (optional).
    pub expiry_date: Option<String>,
    /// Parent IdTag (v1.6 only, ignored in v2.0.1).
    pub parent_id_tag: Option<String>,
}

/// Result of a GetCompositeSchedule command (version-agnostic).
#[derive(Debug)]
pub struct CompositeScheduleResult {
    /// Status: "Accepted", "Rejected", etc.
    pub status: String,
    /// The composite schedule as raw JSON (version-specific structure).
    pub schedule: Option<serde_json::Value>,
    /// Connector ID (v1.6 only).
    pub connector_id: Option<i32>,
    /// Schedule start time as ISO 8601 string (v1.6 only).
    pub schedule_start: Option<String>,
}

// ── Re-exports ─────────────────────────────────────────────────────

pub use dispatcher::{
    create_command_dispatcher, CommandDispatcher, SharedCommandDispatcher,
};

const RESPONSE_TIMEOUT_SECS: u64 = 30;

struct PendingRequest {
    action_name: String,
    response_sender: oneshot::Sender<Result<Value, CommandError>>,
}

#[derive(Debug, Clone)]
pub enum CommandError {
    NotConnected(String),
    SendFailed(String),
    Timeout,
    InvalidResponse(String),
    CallError { code: String, description: String },
    /// The command is not supported by the charge point's OCPP version.
    UnsupportedVersion(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConnected(id) => write!(f, "Charge point not connected: {}", id),
            Self::SendFailed(msg) => write!(f, "Failed to send: {}", msg),
            Self::Timeout => write!(f, "Response timeout"),
            Self::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            Self::CallError { code, description } => {
                write!(f, "CallError {}: {}", code, description)
            }
            Self::UnsupportedVersion(msg) => write!(f, "Unsupported version: {}", msg),
        }
    }
}

impl std::error::Error for CommandError {}

/// Command sender for sending OCPP commands to charge points
pub struct CommandSender {
    session_registry: SharedSessionRegistry,
    pending_requests: DashMap<(String, String), PendingRequest>,
    message_counter: AtomicU64,
}

impl CommandSender {
    pub fn new(session_registry: SharedSessionRegistry) -> Self {
        Self {
            session_registry,
            pending_requests: DashMap::new(),
            message_counter: AtomicU64::new(1),
        }
    }

    fn generate_message_id(&self) -> String {
        let id = self.message_counter.fetch_add(1, Ordering::SeqCst);
        format!("CS-{}", id)
    }

    /// Send an OCPP command to a charge point.
    ///
    /// `action` is the OCPP action name (e.g. "RemoteStopTransaction").
    /// `payload` is the JSON payload for the call.
    ///
    /// Returns the response payload as a `serde_json::Value`.
    pub async fn send_command(
        &self,
        charge_point_id: &str,
        action: &str,
        payload: Value,
    ) -> Result<Value, CommandError> {
        let message_id = self.generate_message_id();

        let frame = OcppFrame::Call {
            unique_id: message_id.clone(),
            action: action.to_string(),
            payload,
        };
        let json = frame.serialize();

        let (tx, rx) = oneshot::channel();

        let key = (charge_point_id.to_string(), message_id.clone());
        self.pending_requests.insert(
            key.clone(),
            PendingRequest {
                action_name: action.to_string(),
                response_sender: tx,
            },
        );

        info!(
            charge_point_id,
            action,
            message_id = message_id.as_str(),
            "Sending command"
        );

        if let Err(e) = self.session_registry.send_to(charge_point_id, json) {
            self.pending_requests.remove(&key);
            return Err(CommandError::NotConnected(e));
        }

        match timeout(Duration::from_secs(RESPONSE_TIMEOUT_SECS), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                self.pending_requests.remove(&key);
                Err(CommandError::InvalidResponse("Channel closed".to_string()))
            }
            Err(_) => {
                self.pending_requests.remove(&key);
                warn!(
                    charge_point_id,
                    action,
                    message_id = message_id.as_str(),
                    "Command timed out"
                );
                Err(CommandError::Timeout)
            }
        }
    }

    pub fn handle_response(&self, charge_point_id: &str, message_id: &str, payload: Value) {
        let key = (charge_point_id.to_string(), message_id.to_string());
        if let Some((_, pending)) = self.pending_requests.remove(&key) {
            info!(
                charge_point_id,
                action = pending.action_name.as_str(),
                message_id,
                "Received response"
            );
            let _ = pending.response_sender.send(Ok(payload));
        } else {
            warn!(charge_point_id, message_id, "Response for unknown request");
        }
    }

    pub fn handle_error(
        &self,
        charge_point_id: &str,
        message_id: &str,
        error_code: &str,
        error_description: &str,
    ) {
        let key = (charge_point_id.to_string(), message_id.to_string());
        if let Some((_, pending)) = self.pending_requests.remove(&key) {
            warn!(
                charge_point_id,
                action = pending.action_name.as_str(),
                message_id,
                error_code,
                error_description,
                "Received error"
            );
            let _ = pending.response_sender.send(Err(CommandError::CallError {
                code: error_code.to_string(),
                description: error_description.to_string(),
            }));
        }
    }

    pub fn cleanup_charge_point(&self, charge_point_id: &str) {
        self.pending_requests
            .retain(|key, _| key.0 != charge_point_id);
    }
}

pub type SharedCommandSender = Arc<CommandSender>;

pub fn create_command_sender(session_registry: SharedSessionRegistry) -> SharedCommandSender {
    Arc::new(CommandSender::new(session_registry))
}
