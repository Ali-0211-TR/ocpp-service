//! Command sender for Central System to Charge Point communication
//!
//! Handles sending OCPP Call messages to charge points and tracking responses.

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
use log::{info, warn};
use tokio::sync::oneshot;
use tokio::time::timeout;

use ocpp_rs::v16::call::{Action, Call};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::parse::{self, Message};

use crate::session::SharedSessionManager;

// Re-exports for convenience
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

/// Timeout for waiting for a response from charge point
const RESPONSE_TIMEOUT_SECS: u64 = 30;

/// Pending request waiting for a response
struct PendingRequest {
    action_name: String,
    response_sender: oneshot::Sender<Result<ResultPayload, CommandError>>,
}

/// Command sender errors
#[derive(Debug, Clone)]
pub enum CommandError {
    /// Charge point not connected
    NotConnected(String),
    /// Failed to send message
    SendFailed(String),
    /// Response timeout
    Timeout,
    /// Invalid response
    InvalidResponse(String),
    /// Charge point returned error
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
    session_manager: SharedSessionManager,
    /// Pending requests indexed by (charge_point_id, message_id)
    pending_requests: DashMap<(String, String), PendingRequest>,
    /// Message ID counter
    message_counter: AtomicU64,
}

impl CommandSender {
    pub fn new(session_manager: SharedSessionManager) -> Self {
        Self {
            session_manager,
            pending_requests: DashMap::new(),
            message_counter: AtomicU64::new(1),
        }
    }

    /// Generate a unique message ID
    fn generate_message_id(&self) -> String {
        let id = self.message_counter.fetch_add(1, Ordering::SeqCst);
        format!("CS-{}", id)
    }

    /// Send a command to a charge point and wait for response
    pub async fn send_command(
        &self,
        charge_point_id: &str,
        action: Action,
    ) -> Result<ResultPayload, CommandError> {
        let message_id = self.generate_message_id();
        let action_name = action.as_ref().to_string();

        // Create the Call message using the constructor
        let call = Call::new(message_id.clone(), action);

        let message = Message::Call(call);
        let json = parse::serialize_message(&message)
            .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {:?}", e)))?;

        // Create channel for response
        let (tx, rx) = oneshot::channel();

        // Register pending request
        let key = (charge_point_id.to_string(), message_id.clone());
        self.pending_requests.insert(
            key.clone(),
            PendingRequest {
                action_name: action_name.clone(),
                response_sender: tx,
            },
        );

        info!(
            "[{}] Sending {} (id: {})",
            charge_point_id, action_name, message_id
        );

        // Send the message
        if let Err(e) = self.session_manager.send_to(charge_point_id, json) {
            self.pending_requests.remove(&key);
            return Err(CommandError::NotConnected(e));
        }

        // Wait for response with timeout
        match timeout(Duration::from_secs(RESPONSE_TIMEOUT_SECS), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                // Channel closed
                self.pending_requests.remove(&key);
                Err(CommandError::InvalidResponse("Channel closed".to_string()))
            }
            Err(_) => {
                // Timeout
                self.pending_requests.remove(&key);
                warn!(
                    "[{}] Command {} timed out (id: {})",
                    charge_point_id, action_name, message_id
                );
                Err(CommandError::Timeout)
            }
        }
    }

    /// Handle an incoming CallResult for a pending request
    pub fn handle_response(&self, charge_point_id: &str, message_id: &str, payload: ResultPayload) {
        let key = (charge_point_id.to_string(), message_id.to_string());

        if let Some((_, pending)) = self.pending_requests.remove(&key) {
            info!(
                "[{}] Received response for {} (id: {})",
                charge_point_id, pending.action_name, message_id
            );
            let _ = pending.response_sender.send(Ok(payload));
        } else {
            warn!(
                "[{}] Received response for unknown request: {}",
                charge_point_id, message_id
            );
        }
    }

    /// Handle an incoming CallError for a pending request
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
                "[{}] Received error for {} (id: {}): {} - {}",
                charge_point_id, pending.action_name, message_id, error_code, error_description
            );
            let _ = pending.response_sender.send(Err(CommandError::CallError {
                code: error_code.to_string(),
                description: error_description.to_string(),
            }));
        }
    }

    /// Clean up pending requests for a disconnected charge point
    pub fn cleanup_charge_point(&self, charge_point_id: &str) {
        self.pending_requests.retain(|key, _| key.0 != charge_point_id);
    }
}

/// Thread-safe command sender
pub type SharedCommandSender = Arc<CommandSender>;

pub fn create_command_sender(session_manager: SharedSessionManager) -> SharedCommandSender {
    Arc::new(CommandSender::new(session_manager))
}
