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

/// SetChargingProfile request body (v1.6 + v2.0.1).
///
/// The `charging_profile` field accepts a raw JSON object matching the
/// OCPP version-specific ChargingProfile schema.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SetChargingProfileRequest {
    /// EVSE/Connector ID (0 = station-wide).
    pub evse_id: i32,
    /// Full ChargingProfile as a JSON object (OCPP 1.6 or 2.0.1 schema).
    pub charging_profile: serde_json::Value,
}

/// GetCompositeSchedule request body.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct GetCompositeScheduleRequest {
    /// Connector/EVSE ID (0 = grid connection).
    pub connector_id: i32,
    /// Duration in seconds for the requested schedule.
    #[validate(range(min = 1, message = "duration must be positive"))]
    pub duration: i32,
    /// Optional charging rate unit: "W" (Watts) or "A" (Amps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging_rate_unit: Option<String>,
}

/// GetCompositeSchedule response.
#[derive(Debug, Serialize, ToSchema)]
pub struct GetCompositeScheduleResponse {
    pub status: String,
    /// The composite schedule as a JSON object (version-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<serde_json::Value>,
    /// Connector ID (v1.6 only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<i32>,
    /// Schedule start time as ISO 8601 string (v1.6 only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_start: Option<String>,
}

// ─── Firmware Management ───────────────────────────────────────────────────

/// UpdateFirmware request body.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateFirmwareRequest {
    /// URI of the firmware image.
    #[validate(length(min = 1, message = "location is required"))]
    pub location: String,
    /// Date/time at which the charge point should retrieve the firmware (ISO 8601).
    #[validate(length(min = 1, message = "retrieve_date is required"))]
    pub retrieve_date: String,
    /// Number of retries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<i32>,
    /// Interval between retries in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_interval: Option<i32>,
}

/// UpdateFirmware response.
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateFirmwareResponse {
    pub status: String,
}

/// GetDiagnostics / GetLog request body.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct GetDiagnosticsRequest {
    /// URI where the charge point should upload diagnostics.
    #[validate(length(min = 1, message = "location is required"))]
    pub location: String,
    /// Number of retries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<i32>,
    /// Interval between retries in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_interval: Option<i32>,
    /// Oldest timestamp for the requested log (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// Latest timestamp for the requested log (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_time: Option<String>,
    /// Log type (v2.0.1 only): "DiagnosticsLog" or "SecurityLog".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_type: Option<String>,
}

/// GetDiagnostics / GetLog response.
#[derive(Debug, Serialize, ToSchema)]
pub struct GetDiagnosticsResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

// ─── Device Reports (v2.0.1 only) ─────────────────────────────────────

/// GetBaseReport request body.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct GetBaseReportRequest {
    /// Report type: "ConfigurationInventory", "FullInventory", or "SummaryInventory".
    #[validate(length(min = 1, message = "report_base is required"))]
    pub report_base: String,
}

/// GetBaseReport response.
#[derive(Debug, Serialize, ToSchema)]
pub struct GetBaseReportResponse {
    pub status: String,
    /// The request_id used to track the report.
    /// Use GET /charge-points/{id}/report?request_id={request_id} to retrieve parts.
    pub request_id: i32,
}

// ─── Variable Monitoring (v2.0.1 only) ────────────────────────────────

/// A single monitor descriptor for SetVariableMonitoring.
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct MonitorDescriptorDto {
    /// Component name (e.g. "EVSE", "Connector").
    #[validate(length(min = 1, message = "component is required"))]
    pub component: String,
    /// Variable name within the component.
    #[validate(length(min = 1, message = "variable is required"))]
    pub variable: String,
    /// Monitor type: "UpperThreshold", "LowerThreshold", "Delta", "Periodic", "PeriodicClockAligned".
    #[validate(length(min = 1, message = "monitor_type is required"))]
    pub monitor_type: String,
    /// Threshold / delta / period value.
    pub value: f64,
    /// Severity (0–9). 0 = Danger, 9 = Informational.
    pub severity: u8,
    /// Whether the monitor applies only during a transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<bool>,
    /// Existing monitor ID to update (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
}

/// SetVariableMonitoring request body (v2.0.1 only).
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SetVariableMonitoringRequest {
    #[validate(length(min = 1, message = "at least one monitor is required"))]
    #[validate(nested)]
    pub monitors: Vec<MonitorDescriptorDto>,
}

/// A single monitoring-set result.
#[derive(Debug, Serialize, ToSchema)]
pub struct MonitoringResultDto {
    pub component: String,
    pub variable: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_id: Option<i32>,
    pub monitor_type: String,
}

/// SetVariableMonitoring response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SetVariableMonitoringResponse {
    pub results: Vec<MonitoringResultDto>,
}

/// SetMonitoringBase request body (v2.0.1 only).
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SetMonitoringBaseRequest {
    /// Monitoring base: "All", "FactoryDefault", or "HardWiredOnly".
    #[validate(length(min = 1, message = "monitoring_base is required"))]
    pub monitoring_base: String,
}

/// SetMonitoringBase response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SetMonitoringBaseResponse {
    pub status: String,
}

/// ClearVariableMonitoring request body (v2.0.1 only).
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ClearVariableMonitoringRequest {
    /// List of monitor IDs to clear.
    #[validate(length(min = 1, message = "at least one monitor ID is required"))]
    pub ids: Vec<i32>,
}

/// A single clear-monitor result.
#[derive(Debug, Serialize, ToSchema)]
pub struct ClearMonitoringResultDto {
    pub id: i32,
    pub status: String,
}

/// ClearVariableMonitoring response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ClearVariableMonitoringResponse {
    pub results: Vec<ClearMonitoringResultDto>,
}

// ─── Charging Profile Queries (v2.0.1) ───────────────────────────────

/// GetChargingProfiles request body (v2.0.1 only).
///
/// Instructs the charge point to report its installed charging profiles.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct GetChargingProfilesHttpRequest {
    /// Request ID to correlate with the asynchronous ReportChargingProfiles responses.
    pub request_id: i32,
    /// EVSE ID filter (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evse_id: Option<i32>,
    /// Filter by charging profile purpose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// Filter by stack level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_level: Option<i32>,
    /// Filter by specific profile IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_ids: Option<Vec<i32>>,
}

/// GetChargingProfiles response.
#[derive(Debug, Serialize, ToSchema)]
pub struct GetChargingProfilesHttpResponse {
    /// Status returned by the charge point ("Accepted" or "NoProfiles").
    pub status: String,
}

/// A stored charging profile DTO (from DB).
#[derive(Debug, Serialize, ToSchema)]
pub struct ChargingProfileDto {
    pub id: i32,
    pub charge_point_id: String,
    pub evse_id: i32,
    pub profile_id: i32,
    pub stack_level: i32,
    pub purpose: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrency_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<String>,
    pub schedule_json: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Response for listing stored charging profiles.
#[derive(Debug, Serialize, ToSchema)]
pub struct ChargingProfileListResponse {
    pub profiles: Vec<ChargingProfileDto>,
}

// ─── Transaction Status (v2.0.1) ─────────────────────────────────────

/// GetTransactionStatus request body (v2.0.1 only).
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct GetTransactionStatusRequest {
    /// Transaction ID to query (optional — omit to query all).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
}

/// GetTransactionStatus response.
#[derive(Debug, Serialize, ToSchema)]
pub struct GetTransactionStatusResponse {
    /// Whether the transaction is still ongoing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ongoing_indicator: Option<bool>,
    /// Whether the station has queued messages to deliver.
    pub messages_in_queue: bool,
}
