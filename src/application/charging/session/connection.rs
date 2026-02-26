//! WebSocket connection abstraction

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

use crate::domain::OcppVersion;

/// Represents an active WebSocket connection to a charge point
#[derive(Debug)]
pub struct Connection {
    /// Unique identifier for this connection instance
    pub connection_id: u64,
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

/// Info returned when an existing session is evicted by a new connection
#[derive(Debug)]
pub struct EvictedSession {
    pub charge_point_id: String,
    pub ocpp_version: OcppVersion,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl Connection {
    pub fn new(
        connection_id: u64,
        charge_point_id: impl Into<String>,
        sender: mpsc::UnboundedSender<String>,
        ocpp_version: OcppVersion,
    ) -> Self {
        let now = Utc::now();
        Self {
            connection_id,
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

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_connection() -> (Connection, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let conn = Connection::new(1, "CP001", tx, OcppVersion::V16);
        (conn, rx)
    }

    #[test]
    fn new_connection_fields() {
        let (conn, _rx) = make_connection();
        assert_eq!(conn.charge_point_id, "CP001");
        assert_eq!(conn.ocpp_version, OcppVersion::V16);
        assert!(conn.connected_at <= Utc::now());
    }

    #[test]
    fn send_delivers_message() {
        let (conn, mut rx) = make_connection();
        conn.send("hello".into()).unwrap();
        assert_eq!(rx.try_recv().unwrap(), "hello");
    }

    #[test]
    fn send_to_closed_channel_returns_error() {
        let (conn, rx) = make_connection();
        drop(rx);
        assert!(conn.send("msg".into()).is_err());
    }

    #[test]
    fn touch_updates_last_activity() {
        let (mut conn, _rx) = make_connection();
        let before = conn.last_activity;
        std::thread::sleep(std::time::Duration::from_millis(10));
        conn.touch();
        assert!(conn.last_activity >= before);
    }

    #[test]
    fn is_stale_with_old_activity() {
        let (mut conn, _rx) = make_connection();
        // Manually set last_activity to 10 seconds ago
        conn.last_activity = Utc::now() - chrono::Duration::seconds(10);
        assert!(conn.is_stale(5));
    }

    #[test]
    fn is_not_stale_with_large_timeout() {
        let (conn, _rx) = make_connection();
        assert!(!conn.is_stale(3600));
    }
}
