//! Command DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RemoteStartRequest {
    pub id_tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_value: Option<f64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RemoteStopRequest {
    pub transaction_id: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetRequest {
    #[serde(rename = "type")]
    pub reset_type: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UnlockConnectorRequest {
    pub connector_id: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangeAvailabilityRequest {
    pub connector_id: u32,
    #[serde(rename = "type")]
    pub availability_type: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TriggerMessageRequest {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommandResponse {
    pub status: String,
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangeConfigurationRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DataTransferRequest {
    pub vendor_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DataTransferResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LocalListVersionResponse {
    pub list_version: i32,
}
