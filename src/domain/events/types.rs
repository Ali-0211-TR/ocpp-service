//! Notification events
//!
//! Defines all event types that can be broadcasted to subscribers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event types for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Event {
    ChargePointConnected(ChargePointConnectedEvent),
    ChargePointDisconnected(ChargePointDisconnectedEvent),
    ChargePointStatusChanged(ChargePointStatusChangedEvent),
    ConnectorStatusChanged(ConnectorStatusChangedEvent),
    TransactionStarted(TransactionStartedEvent),
    TransactionStopped(TransactionStoppedEvent),
    TransactionBilled(TransactionBilledEvent),
    MeterValuesReceived(MeterValuesEvent),
    HeartbeatReceived(HeartbeatEvent),
    AuthorizationResult(AuthorizationEvent),
    BootNotification(BootNotificationEvent),
    DeviceAlert(DeviceAlertEvent),
    Error(ErrorEvent),
}

impl Event {
    pub fn event_type(&self) -> &'static str {
        match self {
            Event::ChargePointConnected(_) => "charge_point_connected",
            Event::ChargePointDisconnected(_) => "charge_point_disconnected",
            Event::ChargePointStatusChanged(_) => "charge_point_status_changed",
            Event::ConnectorStatusChanged(_) => "connector_status_changed",
            Event::TransactionStarted(_) => "transaction_started",
            Event::TransactionStopped(_) => "transaction_stopped",
            Event::TransactionBilled(_) => "transaction_billed",
            Event::MeterValuesReceived(_) => "meter_values_received",
            Event::HeartbeatReceived(_) => "heartbeat_received",
            Event::AuthorizationResult(_) => "authorization_result",
            Event::BootNotification(_) => "boot_notification",
            Event::DeviceAlert(_) => "device_alert",
            Event::Error(_) => "error",
        }
    }

    pub fn charge_point_id(&self) -> Option<&str> {
        match self {
            Event::ChargePointConnected(e) => Some(&e.charge_point_id),
            Event::ChargePointDisconnected(e) => Some(&e.charge_point_id),
            Event::ChargePointStatusChanged(e) => Some(&e.charge_point_id),
            Event::ConnectorStatusChanged(e) => Some(&e.charge_point_id),
            Event::TransactionStarted(e) => Some(&e.charge_point_id),
            Event::TransactionStopped(e) => Some(&e.charge_point_id),
            Event::TransactionBilled(e) => Some(&e.charge_point_id),
            Event::MeterValuesReceived(e) => Some(&e.charge_point_id),
            Event::HeartbeatReceived(e) => Some(&e.charge_point_id),
            Event::AuthorizationResult(e) => Some(&e.charge_point_id),
            Event::BootNotification(e) => Some(&e.charge_point_id),
            Event::DeviceAlert(e) => Some(&e.charge_point_id),
            Event::Error(e) => e.charge_point_id.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargePointConnectedEvent {
    pub charge_point_id: String,
    pub ocpp_version: String,
    pub timestamp: DateTime<Utc>,
    pub remote_addr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargePointDisconnectedEvent {
    pub charge_point_id: String,
    pub timestamp: DateTime<Utc>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargePointStatusChangedEvent {
    pub charge_point_id: String,
    pub old_status: String,
    pub new_status: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorStatusChangedEvent {
    pub charge_point_id: String,
    pub connector_id: u32,
    pub status: String,
    pub error_code: Option<String>,
    pub info: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStartedEvent {
    pub charge_point_id: String,
    pub connector_id: u32,
    pub transaction_id: i32,
    pub id_tag: String,
    pub meter_start: i32,
    pub timestamp: DateTime<Utc>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionBilledEvent {
    pub charge_point_id: String,
    pub transaction_id: i32,
    pub energy_kwh: f64,
    pub duration_minutes: f64,
    pub energy_cost: f64,
    pub time_cost: f64,
    pub session_fee: f64,
    pub total_cost: f64,
    pub currency: String,
    pub tariff_name: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterValuesEvent {
    pub charge_point_id: String,
    pub connector_id: u32,
    pub transaction_id: Option<i32>,
    pub energy_wh: Option<f64>,
    pub energy_consumed_wh: Option<f64>,
    pub power_w: Option<f64>,
    pub soc: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEvent {
    pub charge_point_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationEvent {
    pub charge_point_id: String,
    pub id_tag: String,
    pub status: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootNotificationEvent {
    pub charge_point_id: String,
    pub vendor: String,
    pub model: String,
    pub serial_number: Option<String>,
    pub firmware_version: Option<String>,
    pub ocpp_version: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAlertEvent {
    pub charge_point_id: String,
    pub event_id: i32,
    pub component: String,
    pub variable: String,
    pub actual_value: String,
    pub trigger: String,
    pub event_notification_type: String,
    pub severity: Option<u8>,
    pub tech_code: Option<String>,
    pub tech_info: Option<String>,
    pub cleared: Option<bool>,
    pub transaction_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

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
