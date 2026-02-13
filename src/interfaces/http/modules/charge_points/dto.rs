//! Charge Point DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::{ChargePoint, Connector, ConnectorStatus};

/// Charge Point API representation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChargePointDto {
    pub id: String,
    /// OCPP protocol version: "1.6", "2.0.1", "2.1"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocpp_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iccid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imsi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_serial_number: Option<String>,
    pub status: String,
    pub is_online: bool,
    pub connectors: Vec<ConnectorDto>,
    pub registered_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat: Option<DateTime<Utc>>,
}

impl ChargePointDto {
    pub fn from_domain(cp: ChargePoint, is_online: bool) -> Self {
        Self {
            id: cp.id,
            ocpp_version: cp.ocpp_version.map(|v| v.version_string().to_string()),
            vendor: cp.vendor,
            model: cp.model,
            serial_number: cp.serial_number,
            firmware_version: cp.firmware_version,
            iccid: cp.iccid,
            imsi: cp.imsi,
            meter_type: cp.meter_type,
            meter_serial_number: cp.meter_serial_number,
            status: cp.status.to_string(),
            is_online,
            connectors: cp
                .connectors
                .into_iter()
                .map(ConnectorDto::from_domain)
                .collect(),
            registered_at: cp.registered_at,
            last_heartbeat: cp.last_heartbeat,
        }
    }
}

/// Connector (plug) DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ConnectorDto {
    pub id: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_info: Option<String>,
}

impl ConnectorDto {
    pub fn from_domain(connector: Connector) -> Self {
        Self {
            id: connector.id,
            status: connector_status_to_string(&connector.status),
            error_code: connector.error_code,
            error_info: connector.info,
        }
    }
}

fn connector_status_to_string(status: &ConnectorStatus) -> String {
    match status {
        ConnectorStatus::Available => "Available",
        ConnectorStatus::Preparing => "Preparing",
        ConnectorStatus::Charging => "Charging",
        ConnectorStatus::SuspendedEV => "SuspendedEV",
        ConnectorStatus::SuspendedEVSE => "SuspendedEVSE",
        ConnectorStatus::Finishing => "Finishing",
        ConnectorStatus::Reserved => "Reserved",
        ConnectorStatus::Unavailable => "Unavailable",
        ConnectorStatus::Faulted => "Faulted",
    }
    .to_string()
}

/// Request to create a new connector
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateConnectorRequest {
    #[validate(range(min = 1, message = "connector_id must be ≥ 1"))]
    pub connector_id: u32,
}

/// Charge point statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChargePointStats {
    pub total: u32,
    pub online: u32,
    pub offline: u32,
    pub charging: u32,
}

/// Request to set or update a charge point's WebSocket authentication password.
///
/// Used for OCPP Security Profile 1 (Basic Auth).
/// The charge point will authenticate with `Authorization: Basic base64(id:password)`.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SetPasswordRequest {
    /// The new password for the charge point (min 8 characters).
    /// Set to `null` or omit to remove the password (disable Basic Auth for this CP).
    #[validate(length(min = 8, max = 128, message = "password must be 8–128 characters"))]
    pub password: String,
}

/// Response after setting a charge point password.
#[derive(Debug, Serialize, ToSchema)]
pub struct SetPasswordResponse {
    pub message: String,
    /// Whether the charge point now has a password configured.
    pub has_password: bool,
}
