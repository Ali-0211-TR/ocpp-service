//! Session registry — manages active charge point WebSocket connections

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::domain::OcppVersion;

use super::connection::Connection;

/// Thread-safe registry of active OCPP charge point sessions
pub struct SessionRegistry {
    sessions: DashMap<String, Connection>,
}

/// Shared, reference-counted session registry
pub type SharedSessionRegistry = Arc<SessionRegistry>;

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    /// Wrap in `Arc` for shared ownership
    pub fn shared() -> SharedSessionRegistry {
        Arc::new(Self::new())
    }

    /// Register a new charge point connection
    pub fn register(
        &self,
        charge_point_id: &str,
        sender: mpsc::UnboundedSender<String>,
        ocpp_version: OcppVersion,
    ) {
        info!(charge_point_id, %ocpp_version, "Registering charge point session");
        let connection = Connection::new(charge_point_id, sender, ocpp_version);
        self.sessions
            .insert(charge_point_id.to_string(), connection);
    }

    /// Unregister a charge point connection
    pub fn unregister(&self, charge_point_id: &str) {
        if self.sessions.remove(charge_point_id).is_some() {
            info!(charge_point_id, "Unregistered charge point session");
        } else {
            warn!(charge_point_id, "Attempted to unregister unknown session");
        }
    }

    /// Send a message to a specific charge point
    pub fn send_to(&self, charge_point_id: &str, message: String) -> Result<(), String> {
        match self.sessions.get(charge_point_id) {
            Some(conn) => conn.send(message),
            None => Err(format!("Charge point {} not connected", charge_point_id)),
        }
    }

    /// Update last activity for a charge point
    pub fn touch(&self, charge_point_id: &str) {
        if let Some(mut conn) = self.sessions.get_mut(charge_point_id) {
            conn.touch();
        }
    }

    /// Check if a charge point is currently connected
    pub fn is_connected(&self, charge_point_id: &str) -> bool {
        self.sessions.contains_key(charge_point_id)
    }

    /// Get all connected charge point IDs
    pub fn connected_ids(&self) -> Vec<String> {
        self.sessions.iter().map(|r| r.key().clone()).collect()
    }

    /// Broadcast a message to all connected charge points
    pub fn broadcast(&self, message: &str) {
        for entry in self.sessions.iter() {
            if let Err(e) = entry.send(message.to_string()) {
                warn!(
                    charge_point_id = entry.charge_point_id.as_str(),
                    error = %e,
                    "Failed to broadcast to charge point"
                );
            }
        }
    }

    /// Number of active sessions
    pub fn count(&self) -> usize {
        self.sessions.len()
    }

    /// Get the negotiated OCPP version for a charge point
    pub fn get_version(&self, charge_point_id: &str) -> Option<OcppVersion> {
        self.sessions
            .get(charge_point_id)
            .map(|conn| conn.ocpp_version)
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
