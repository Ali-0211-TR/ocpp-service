//! Session manager - manages active connections

use std::sync::Arc;

use dashmap::DashMap;
use log::info;
use tokio::sync::mpsc;

use super::Connection;

/// Manages active WebSocket sessions
pub struct SessionManager {
    /// Active connections indexed by charge point ID
    connections: DashMap<String, Connection>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    /// Register a new connection
    pub fn register(
        &self,
        charge_point_id: impl Into<String>,
        sender: mpsc::UnboundedSender<String>,
    ) -> String {
        let id = charge_point_id.into();
        let connection = Connection::new(id.clone(), sender);
        info!("Session registered: {}", id);
        self.connections.insert(id.clone(), connection);
        id
    }

    /// Unregister a connection
    pub fn unregister(&self, charge_point_id: &str) {
        if self.connections.remove(charge_point_id).is_some() {
            info!("Session unregistered: {}", charge_point_id);
        }
    }

    /// Send a message to a specific charge point
    pub fn send_to(&self, charge_point_id: &str, message: String) -> Result<(), String> {
        if let Some(conn) = self.connections.get(charge_point_id) {
            conn.send(message)
        } else {
            Err(format!("Charge point not connected: {}", charge_point_id))
        }
    }

    /// Update activity timestamp for a connection
    pub fn touch(&self, charge_point_id: &str) {
        if let Some(mut conn) = self.connections.get_mut(charge_point_id) {
            conn.touch();
        }
    }

    /// Check if a charge point is connected
    pub fn is_connected(&self, charge_point_id: &str) -> bool {
        self.connections.contains_key(charge_point_id)
    }

    /// Get list of connected charge point IDs
    pub fn connected_ids(&self) -> Vec<String> {
        self.connections.iter().map(|e| e.key().clone()).collect()
    }

    /// Get number of active connections
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Broadcast a message to all connected charge points
    pub fn broadcast(&self, message: &str) {
        for conn in self.connections.iter() {
            let _ = conn.send(message.to_string());
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe session manager
pub type SharedSessionManager = Arc<SessionManager>;

pub fn create_session_manager() -> SharedSessionManager {
    Arc::new(SessionManager::new())
}
