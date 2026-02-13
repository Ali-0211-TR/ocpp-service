//! Command DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RemoteStartRequest {
    #[validate(length(min = 1, max = 20, message = "id_tag must be 1–20 characters"))]
    pub id_tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_value: Option<f64>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RemoteStopRequest {
    #[validate(range(min = 1, message = "transaction_id must be positive"))]
    pub transaction_id: i32,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetRequest {
    #[validate(length(min = 1, message = "reset type is required"))]
    #[serde(rename = "type")]
    pub reset_type: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UnlockConnectorRequest {
    #[validate(range(min = 1, message = "connector_id must be ≥ 1"))]
    pub connector_id: u32,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangeAvailabilityRequest {
    pub connector_id: u32,
    #[validate(length(min = 1, message = "availability type is required"))]
    #[serde(rename = "type")]
    pub availability_type: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TriggerMessageRequest {
    #[validate(length(min = 1, message = "message type is required"))]
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

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangeConfigurationRequest {
    #[validate(length(min = 1, max = 500, message = "key is required"))]
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct DataTransferRequest {
    #[validate(length(min = 1, max = 255, message = "vendor_id is required"))]
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

/// A single authorization entry to add to the local list.
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct AuthorizationEntryDto {
    /// The RFID / IdTag identifier (1–20 chars for v1.6, up to 36 for v2.0.1).
    #[validate(length(min = 1, max = 36, message = "id_tag is required"))]
    pub id_tag: String,
    /// Authorization status: Accepted, Blocked, Expired, Invalid. Defaults to Accepted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// ISO 8601 expiry date-time (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
    /// Parent IdTag (v1.6 only, ignored in v2.0.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id_tag: Option<String>,
}

/// SendLocalList request body.
///
/// Sends a full or differential update of the local authorization list
/// to the charge point. Works with both OCPP 1.6 and 2.0.1.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SendLocalListRequest {
    /// The version number of the list after this update.
    pub list_version: i32,
    /// Update type: "Full" or "Differential".
    #[validate(length(min = 1, message = "update_type is required (Full or Differential)"))]
    pub update_type: String,
    /// Authorization entries. Empty for Full = clear list.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(nested)]
    pub local_authorization_list: Option<Vec<AuthorizationEntryDto>>,
}

/// SendLocalList response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SendLocalListResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ── v2.0.1-specific DTOs ───────────────────────────────────────────

/// A single variable to query — (component, variable) pair.
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct VariableSelector {
    /// Component name (e.g. "ChargingStation", "EVSE").
    #[validate(length(min = 1, message = "component name is required"))]
    pub component: String,
    /// Variable name within the component.
    #[validate(length(min = 1, message = "variable name is required"))]
    pub variable: String,
}

/// GetVariables request body (v2.0.1 only).
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct GetVariablesRequest {
    #[validate(length(min = 1, message = "at least one variable selector is required"))]
    #[validate(nested)]
    pub variables: Vec<VariableSelector>,
}

/// A single variable result from GetVariables.
#[derive(Debug, Serialize, ToSchema)]
pub struct VariableResultDto {
    pub component: String,
    pub variable: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// GetVariables response.
#[derive(Debug, Serialize, ToSchema)]
pub struct GetVariablesResponse {
    pub results: Vec<VariableResultDto>,
}

/// A single variable to set — (component, variable, value) triple.
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct VariableAssignment {
    /// Component name.
    #[validate(length(min = 1, message = "component name is required"))]
    pub component: String,
    /// Variable name.
    #[validate(length(min = 1, message = "variable name is required"))]
    pub variable: String,
    /// New value.
    pub value: String,
}

/// SetVariables request body (v2.0.1 only).
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SetVariablesRequest {
    #[validate(length(min = 1, message = "at least one variable assignment is required"))]
    #[validate(nested)]
    pub variables: Vec<VariableAssignment>,
}

/// A single set-variable result.
#[derive(Debug, Serialize, ToSchema)]
pub struct SetVariableStatusDto {
    pub component: String,
    pub variable: String,
    pub status: String,
}

/// SetVariables response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SetVariablesResponse {
    pub results: Vec<SetVariableStatusDto>,
}

/// ClearChargingProfile request body (v2.0.1 only).
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ClearChargingProfileRequest {
    /// Clear a specific profile by ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging_profile_id: Option<i32>,
    /// EVSE ID (0 = entire station).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evse_id: Option<i32>,
    /// Filter by charging profile purpose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging_profile_purpose: Option<String>,
    /// Filter by stack level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_level: Option<i32>,
}

/// SetChargingProfile request body (v2.0.1 only).
///
/// The `charging_profile` field accepts a raw JSON object matching the
/// OCPP 2.0.1 `ChargingProfileType` schema.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SetChargingProfileRequest {
    /// EVSE ID to apply the profile to (0 = station-wide).
    pub evse_id: i32,
    /// Full ChargingProfile as a JSON object (OCPP 2.0.1 ChargingProfileType).
    pub charging_profile: serde_json::Value,
}
