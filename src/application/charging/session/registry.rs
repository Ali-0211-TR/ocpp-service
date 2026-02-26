//! Session registry — manages active charge point WebSocket connections

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::domain::OcppVersion;

use super::connection::{Connection, EvictedSession};

/// Minimum interval between reconnections from the same charge point (seconds).
const RECONNECT_DEBOUNCE_SECS: i64 = 2;

/// Outcome of a registration attempt
pub enum RegisterResult {
    /// Fresh connection — no previous session existed
    New { connection_id: u64 },
    /// Replaced an existing session (old sender was dropped)
    Evicted { evicted: EvictedSession, connection_id: u64 },
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
    /// Monotonically increasing connection ID counter
    next_connection_id: AtomicU64,
}

/// Shared, reference-counted session registry
pub type SharedSessionRegistry = Arc<SessionRegistry>;

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            last_disconnect: DashMap::new(),
            next_connection_id: AtomicU64::new(1),
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
        let connection_id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        info!(charge_point_id, %ocpp_version, connection_id, "Registering charge point session");
        let connection = Connection::new(connection_id, charge_point_id, sender, ocpp_version);
        self.sessions
            .insert(charge_point_id.to_string(), connection);

        // Clear debounce timestamp (fresh session is now active)
        self.last_disconnect.remove(charge_point_id);

        // Update Prometheus gauge
        metrics::gauge!("ocpp_connected_stations").set(self.sessions.len() as f64);

        match evicted {
            Some(ev) => RegisterResult::Evicted { evicted: ev, connection_id },
            None => RegisterResult::New { connection_id },
        }
    }

    /// Unregister a charge point connection and record disconnect time for debounce.
    ///
    /// Only removes the session if the `connection_id` matches the current session.
    /// This prevents a stale/evicted connection's cleanup from removing a newer session.
    pub fn unregister(&self, charge_point_id: &str, connection_id: u64) {
        let removed = self.sessions.remove_if(charge_point_id, |_, conn| {
            conn.connection_id == connection_id
        });
        if removed.is_some() {
            self.last_disconnect
                .insert(charge_point_id.to_string(), Utc::now());
            // Update Prometheus gauge
            metrics::gauge!("ocpp_connected_stations").set(self.sessions.len() as f64);
            info!(charge_point_id, connection_id, "Unregistered charge point session");
        }
        // If connection_id doesn't match, a newer session exists — do NOT remove it
    }

    /// Force-unregister a charge point regardless of connection_id (for graceful shutdown).
    pub fn force_unregister(&self, charge_point_id: &str) {
        if self.sessions.remove(charge_point_id).is_some() {
            metrics::gauge!("ocpp_connected_stations").set(self.sessions.len() as f64);
            info!(charge_point_id, "Force-unregistered charge point session");
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

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn make_sender() -> mpsc::UnboundedSender<String> {
        let (tx, _rx) = mpsc::unbounded_channel();
        tx
    }

    #[test]
    fn register_new_session() {
        let reg = SessionRegistry::new();
        let result = reg.register("CP001", make_sender(), OcppVersion::V16);
        assert!(matches!(result, RegisterResult::New { .. }));
        assert_eq!(reg.count(), 1);
        assert!(reg.is_connected("CP001"));
    }

    #[test]
    fn register_evicts_existing_session() {
        let reg = SessionRegistry::new();
        reg.register("CP001", make_sender(), OcppVersion::V16);
        let result = reg.register("CP001", make_sender(), OcppVersion::V16);
        assert!(matches!(result, RegisterResult::Evicted { .. }));
        assert_eq!(reg.count(), 1);
    }

    #[test]
    fn unregister_removes_session() {
        let reg = SessionRegistry::new();
        let result = reg.register("CP001", make_sender(), OcppVersion::V16);
        let conn_id = match result {
            RegisterResult::New { connection_id } => connection_id,
            _ => panic!("expected New"),
        };
        reg.unregister("CP001", conn_id);
        assert_eq!(reg.count(), 0);
        assert!(!reg.is_connected("CP001"));
    }

    #[test]
    fn unregister_nonexistent_is_noop() {
        let reg = SessionRegistry::new();
        reg.unregister("CP_UNKNOWN", 999); // should not panic
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn connected_ids() {
        let reg = SessionRegistry::new();
        reg.register("CP001", make_sender(), OcppVersion::V16);
        reg.register("CP002", make_sender(), OcppVersion::V201);
        let mut ids = reg.connected_ids();
        ids.sort();
        assert_eq!(ids, vec!["CP001", "CP002"]);
    }

    #[test]
    fn get_version() {
        let reg = SessionRegistry::new();
        reg.register("CP001", make_sender(), OcppVersion::V201);
        assert_eq!(reg.get_version("CP001"), Some(OcppVersion::V201));
        assert_eq!(reg.get_version("CP_UNKNOWN"), None);
    }

    #[test]
    fn send_to_connected_charge_point() {
        let reg = SessionRegistry::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        reg.register("CP001", tx, OcppVersion::V16);

        reg.send_to("CP001", "hello".into()).unwrap();
        assert_eq!(rx.try_recv().unwrap(), "hello");
    }

    #[test]
    fn send_to_disconnected_returns_error() {
        let reg = SessionRegistry::new();
        let result = reg.send_to("CP_UNKNOWN", "msg".into());
        assert!(result.is_err());
    }

    #[test]
    fn broadcast_sends_to_all() {
        let reg = SessionRegistry::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        reg.register("CP001", tx1, OcppVersion::V16);
        reg.register("CP002", tx2, OcppVersion::V16);

        reg.broadcast("ping");
        assert_eq!(rx1.try_recv().unwrap(), "ping");
        assert_eq!(rx2.try_recv().unwrap(), "ping");
    }

    #[test]
    fn touch_updates_activity() {
        let reg = SessionRegistry::new();
        reg.register("CP001", make_sender(), OcppVersion::V16);
        let before = reg
            .sessions
            .get("CP001")
            .map(|c| c.last_activity)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        reg.touch("CP001");
        let after = reg
            .sessions
            .get("CP001")
            .map(|c| c.last_activity)
            .unwrap();
        assert!(after >= before);
    }

    #[test]
    fn debounce_rejects_fast_reconnect() {
        let reg = SessionRegistry::new();
        let result = reg.register("CP001", make_sender(), OcppVersion::V16);
        let conn_id = match result {
            RegisterResult::New { connection_id } => connection_id,
            _ => panic!("expected New"),
        };
        reg.unregister("CP001", conn_id); // records disconnect time

        // Immediately try to reconnect — should be debounced
        let result = reg.register("CP001", make_sender(), OcppVersion::V16);
        assert!(matches!(result, RegisterResult::Debounced { .. }));
    }

    #[test]
    fn evicted_session_cleanup_does_not_remove_new_session() {
        let reg = SessionRegistry::new();
        // First connection
        let result = reg.register("CP001", make_sender(), OcppVersion::V16);
        let old_conn_id = match result {
            RegisterResult::New { connection_id } => connection_id,
            _ => panic!("expected New"),
        };

        // Second connection evicts first
        let result = reg.register("CP001", make_sender(), OcppVersion::V16);
        let new_conn_id = match result {
            RegisterResult::Evicted { connection_id, .. } => connection_id,
            _ => panic!("expected Evicted"),
        };

        // Old connection's cleanup tries to unregister — should be a no-op
        reg.unregister("CP001", old_conn_id);
        assert_eq!(reg.count(), 1); // new session still alive!
        assert!(reg.is_connected("CP001"));

        // New connection's cleanup works correctly
        reg.unregister("CP001", new_conn_id);
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn shared_creates_arc() {
        let shared = SessionRegistry::shared();
        assert_eq!(shared.count(), 0);
    }

    #[test]
    fn default_creates_registry() {
        let reg = SessionRegistry::default();
        assert_eq!(reg.count(), 0);
    }
}
