//! WebSocket connection abstraction

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

use crate::domain::OcppVersion;

/// Represents an active WebSocket connection to a charge point
#[derive(Debug)]
pub struct Connection {
    /// Charge point ID
    pub charge_point_id: String,
    /// Channel to send messages to the charge point
    pub sender: mpsc::UnboundedSender<String>,
    /// Negotiated OCPP protocol version for this connection
    pub ocpp_version: OcppVersion,
    /// When the connection was established
    pub connected_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

impl Connection {
    pub fn new(
        charge_point_id: impl Into<String>,
        sender: mpsc::UnboundedSender<String>,
        ocpp_version: OcppVersion,
    ) -> Self {
        let now = Utc::now();
        Self {
            charge_point_id: charge_point_id.into(),
            sender,
            ocpp_version,
            connected_at: now,
            last_activity: now,
        }
    }

    /// Send a message to the charge point
    pub fn send(&self, message: String) -> Result<(), String> {
        self.sender
            .send(message)
            .map_err(|e| format!("Failed to send message: {}", e))
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Check if connection is considered stale
    pub fn is_stale(&self, timeout_seconds: i64) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_activity)
            .num_seconds();
        elapsed > timeout_seconds
    }
}
