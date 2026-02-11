//! Command sender for Central System to Charge Point communication

pub mod change_availability;
pub mod change_configuration;
pub mod clear_cache;
pub mod data_transfer;
pub mod get_configuration;
pub mod get_local_list_version;
pub mod remote_start;
pub mod remote_stop;
pub mod reset;
pub mod trigger_message;
pub mod unlock_connector;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{info, warn};

use crate::application::session::SharedSessionRegistry;
use crate::support::ocpp_frame::OcppFrame;

pub use change_availability::{change_availability, Availability};
pub use change_configuration::change_configuration;
pub use clear_cache::clear_cache;
pub use data_transfer::{data_transfer, DataTransferResult};
pub use get_configuration::{get_configuration, ConfigurationResult};
pub use get_local_list_version::get_local_list_version;
pub use remote_start::remote_start_transaction;
pub use remote_stop::remote_stop_transaction;
pub use reset::{reset, ResetKind};
pub use trigger_message::{trigger_message, TriggerType};
pub use unlock_connector::unlock_connector;

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

    pub fn handle_response(
        &self,
        charge_point_id: &str,
        message_id: &str,
        payload: Value,
    ) {
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
