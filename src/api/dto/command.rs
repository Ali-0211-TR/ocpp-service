//! Command DTOs for sending commands to charge points

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Remote start transaction request
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "id_tag": "RFID001",
    "connector_id": 1
}))]
pub struct RemoteStartRequest {
    /// RFID tag or identifier for authorization
    pub id_tag: String,
    /// Optional connector ID (1-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<u32>,
}

/// Remote stop transaction request
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "transaction_id": 123
}))]
pub struct RemoteStopRequest {
    /// Transaction ID to stop
    pub transaction_id: i32,
}

/// Reset request
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "type": "Soft"
}))]
pub struct ResetRequest {
    /// Reset type: "Soft" or "Hard"
    #[serde(rename = "type")]
    pub reset_type: String,
}

/// Unlock connector request
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "connector_id": 1
}))]
pub struct UnlockConnectorRequest {
    /// Connector ID to unlock (1-based)
    pub connector_id: u32,
}

/// Change availability request
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "connector_id": 0,
    "type": "Operative"
}))]
pub struct ChangeAvailabilityRequest {
    /// Connector ID (0 = entire charge point)
    pub connector_id: u32,
    /// Availability type: "Operative" or "Inoperative"
    #[serde(rename = "type")]
    pub availability_type: String,
}

/// Trigger message request
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "message": "StatusNotification",
    "connector_id": 1
}))]
pub struct TriggerMessageRequest {
    /// Message type: "BootNotification", "Heartbeat", "StatusNotification", etc.
    pub message: String,
    /// Optional connector ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<u32>,
}

/// Command response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "status": "Accepted",
    "message": "Command sent successfully"
}))]
pub struct CommandResponse {
    /// Response status from charge point
    pub status: String,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl CommandResponse {
    pub fn accepted() -> Self {
        Self {
            status: "Accepted".to_string(),
            message: Some("Command sent successfully".to_string()),
        }
    }

    pub fn rejected(reason: impl Into<String>) -> Self {
        Self {
            status: "Rejected".to_string(),
            message: Some(reason.into()),
        }
    }

    pub fn from_status(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            message: None,
        }
    }
}
