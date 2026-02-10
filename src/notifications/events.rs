//! Notification events
//!
//! Defines all event types that can be broadcasted to WebSocket clients.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event types for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Event {
    /// Charge point connected via WebSocket
    ChargePointConnected(ChargePointConnectedEvent),
    /// Charge point disconnected
    ChargePointDisconnected(ChargePointDisconnectedEvent),
    /// Charge point status changed (Online/Offline)
    ChargePointStatusChanged(ChargePointStatusChangedEvent),
    /// Connector status changed
    ConnectorStatusChanged(ConnectorStatusChangedEvent),
    /// Transaction started
    TransactionStarted(TransactionStartedEvent),
    /// Transaction stopped
    TransactionStopped(TransactionStoppedEvent),
    /// Meter values received
    MeterValuesReceived(MeterValuesEvent),
    /// Heartbeat received
    HeartbeatReceived(HeartbeatEvent),
    /// Authorization result
    AuthorizationResult(AuthorizationEvent),
    /// Boot notification received
    BootNotification(BootNotificationEvent),
    /// Error occurred
    Error(ErrorEvent),
}

impl Event {
    /// Get the event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            Event::ChargePointConnected(_) => "charge_point_connected",
            Event::ChargePointDisconnected(_) => "charge_point_disconnected",
            Event::ChargePointStatusChanged(_) => "charge_point_status_changed",
            Event::ConnectorStatusChanged(_) => "connector_status_changed",
            Event::TransactionStarted(_) => "transaction_started",
            Event::TransactionStopped(_) => "transaction_stopped",
            Event::MeterValuesReceived(_) => "meter_values_received",
            Event::HeartbeatReceived(_) => "heartbeat_received",
            Event::AuthorizationResult(_) => "authorization_result",
            Event::BootNotification(_) => "boot_notification",
            Event::Error(_) => "error",
        }
    }

    /// Get the charge point ID if applicable
    pub fn charge_point_id(&self) -> Option<&str> {
        match self {
            Event::ChargePointConnected(e) => Some(&e.charge_point_id),
            Event::ChargePointDisconnected(e) => Some(&e.charge_point_id),
            Event::ChargePointStatusChanged(e) => Some(&e.charge_point_id),
            Event::ConnectorStatusChanged(e) => Some(&e.charge_point_id),
            Event::TransactionStarted(e) => Some(&e.charge_point_id),
            Event::TransactionStopped(e) => Some(&e.charge_point_id),
            Event::MeterValuesReceived(e) => Some(&e.charge_point_id),
            Event::HeartbeatReceived(e) => Some(&e.charge_point_id),
            Event::AuthorizationResult(e) => Some(&e.charge_point_id),
            Event::BootNotification(e) => Some(&e.charge_point_id),
            Event::Error(e) => e.charge_point_id.as_deref(),
        }
    }
}

/// Charge point connected event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargePointConnectedEvent {
    pub charge_point_id: String,
    pub timestamp: DateTime<Utc>,
    pub remote_addr: Option<String>,
}

/// Charge point disconnected event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargePointDisconnectedEvent {
    pub charge_point_id: String,
    pub timestamp: DateTime<Utc>,
    pub reason: Option<String>,
}

/// Charge point status changed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargePointStatusChangedEvent {
    pub charge_point_id: String,
    pub old_status: String,
    pub new_status: String,
    pub timestamp: DateTime<Utc>,
}

/// Connector status changed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorStatusChangedEvent {
    pub charge_point_id: String,
    pub connector_id: u32,
    pub status: String,
    pub error_code: Option<String>,
    pub info: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Transaction started event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStartedEvent {
    pub charge_point_id: String,
    pub connector_id: u32,
    pub transaction_id: i32,
    pub id_tag: String,
    pub meter_start: i32,
    pub timestamp: DateTime<Utc>,
}

/// Transaction stopped event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStoppedEvent {
    pub charge_point_id: String,
    pub transaction_id: i32,
    pub id_tag: Option<String>,
    pub meter_stop: i32,
    pub energy_consumed_kwh: f64,
    pub total_cost: f64,
    pub currency: String,
    pub reason: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Meter values event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterValuesEvent {
    pub charge_point_id: String,
    pub connector_id: u32,
    pub transaction_id: Option<i32>,
    /// Current energy meter reading in Wh
    pub energy_wh: Option<f64>,
    /// Energy consumed since start of transaction in Wh
    pub energy_consumed_wh: Option<f64>,
    /// Current charging power in W
    pub power_w: Option<f64>,
    /// Current State of Charge in %
    pub soc: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

/// Heartbeat event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEvent {
    pub charge_point_id: String,
    pub timestamp: DateTime<Utc>,
}

/// Authorization event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationEvent {
    pub charge_point_id: String,
    pub id_tag: String,
    pub status: String, // Accepted, Blocked, Expired, Invalid, ConcurrentTx
    pub timestamp: DateTime<Utc>,
}

/// Boot notification event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootNotificationEvent {
    pub charge_point_id: String,
    pub vendor: String,
    pub model: String,
    pub serial_number: Option<String>,
    pub firmware_version: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Error event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub charge_point_id: Option<String>,
    pub error_type: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Wrapper for sending events with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMessage {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    #[serde(flatten)]
    pub event: Event,
}

impl EventMessage {
    pub fn new(event: Event) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event,
        }
    }
}
