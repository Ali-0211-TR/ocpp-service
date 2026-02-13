//! Session registry — manages active charge point WebSocket connections

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::domain::OcppVersion;

use super::connection::{Connection, EvictedSession};

/// Minimum interval between reconnections from the same charge point (seconds).
const RECONNECT_DEBOUNCE_SECS: i64 = 5;

/// Outcome of a registration attempt
pub enum RegisterResult {
    /// Fresh connection — no previous session existed
    New,
    /// Replaced an existing session (old sender was dropped)
    Evicted(EvictedSession),
    /// Rejected because the charge point reconnected too quickly
    Debounced {
        last_connected_at: DateTime<Utc>,
        seconds_remaining: i64,
    },
}

/// Thread-safe registry of active OCPP charge point sessions
pub struct SessionRegistry {
    sessions: DashMap<String, Connection>,
    /// Tracks when a charge point was last disconnected (for debounce)
    last_disconnect: DashMap<String, DateTime<Utc>>,
}

/// Shared, reference-counted session registry
pub type SharedSessionRegistry = Arc<SessionRegistry>;

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            last_disconnect: DashMap::new(),
        }
    }

    /// Wrap in `Arc` for shared ownership
    pub fn shared() -> SharedSessionRegistry {
        Arc::new(Self::new())
    }

    /// Register a new charge point connection.
    ///
    /// If a session already exists for this charge point:
    ///   - The old sender channel is dropped (causes the old send_task to stop)
    ///   - Returns `RegisterResult::Evicted` so the caller can publish a disconnect event
    ///
    /// If the charge point reconnects within `RECONNECT_DEBOUNCE_SECS`:
    ///   - Returns `RegisterResult::Debounced` — caller should reject the connection
    pub fn register(
        &self,
        charge_point_id: &str,
        sender: mpsc::UnboundedSender<String>,
        ocpp_version: OcppVersion,
    ) -> RegisterResult {
        // ── Debounce check ─────────────────────────────────
        if let Some(last_dc) = self.last_disconnect.get(charge_point_id) {
            let elapsed = Utc::now().signed_duration_since(*last_dc).num_seconds();
            if elapsed < RECONNECT_DEBOUNCE_SECS {
                let remaining = RECONNECT_DEBOUNCE_SECS - elapsed;
                warn!(
                    charge_point_id,
                    elapsed_seconds = elapsed,
                    debounce_seconds = RECONNECT_DEBOUNCE_SECS,
                    "Reconnection too fast — debouncing"
                );
                return RegisterResult::Debounced {
                    last_connected_at: *last_dc,
                    seconds_remaining: remaining,
                };
            }
        }

        // ── Evict existing session if present ──────────────
        let evicted = self
            .sessions
            .remove(charge_point_id)
            .map(|(_, old_conn)| {
                warn!(
                    charge_point_id,
                    old_version = %old_conn.ocpp_version,
                    connected_since = %old_conn.connected_at,
                    last_activity = %old_conn.last_activity,
                    "Evicting stale session — new connection replaces old"
                );
                // Dropping `old_conn.sender` closes the channel →
                // the old send_task's `rx.recv()` returns None → task exits
                EvictedSession {
                    charge_point_id: old_conn.charge_point_id,
                    ocpp_version: old_conn.ocpp_version,
                    connected_at: old_conn.connected_at,
                    last_activity: old_conn.last_activity,
                }
            });

        // ── Insert new session ─────────────────────────────
        info!(charge_point_id, %ocpp_version, "Registering charge point session");
        let connection = Connection::new(charge_point_id, sender, ocpp_version);
        self.sessions
            .insert(charge_point_id.to_string(), connection);

        // Clear debounce timestamp (fresh session is now active)
        self.last_disconnect.remove(charge_point_id);

        // Update Prometheus gauge
        metrics::gauge!("ocpp_connected_stations").set(self.sessions.len() as f64);

        match evicted {
            Some(ev) => RegisterResult::Evicted(ev),
            None => RegisterResult::New,
        }
    }

    /// Unregister a charge point connection and record disconnect time for debounce.
    pub fn unregister(&self, charge_point_id: &str) {
        if self.sessions.remove(charge_point_id).is_some() {
            self.last_disconnect
                .insert(charge_point_id.to_string(), Utc::now());
            // Update Prometheus gauge
            metrics::gauge!("ocpp_connected_stations").set(self.sessions.len() as f64);
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
