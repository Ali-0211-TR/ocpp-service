//! Charge Point DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::{ChargePoint, Connector, ConnectorStatus};

/// Charge point response DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": "CP001",
    "vendor": "AVT-Company",
    "model": "AVT-Express",
    "serial_number": "avt.001.13.1",
    "firmware_version": "0.9.87",
    "status": "Accepted",
    "is_online": true,
    "connectors": [],
    "registered_at": "2024-01-15T10:30:00Z",
    "last_heartbeat": "2024-01-15T12:00:00Z"
}))]
pub struct ChargePointDto {
    pub id: String,
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
            vendor: cp.vendor,
            model: cp.model,
            serial_number: cp.serial_number,
            firmware_version: cp.firmware_version,
            iccid: cp.iccid,
            imsi: cp.imsi,
            status: cp.status.to_string(),
            is_online,
            connectors: cp.connectors.into_iter().map(ConnectorDto::from_domain).collect(),
            registered_at: cp.registered_at,
            last_heartbeat: cp.last_heartbeat,
        }
    }
}

/// Connector response DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": 1,
    "status": "Available",
    "error_code": null
}))]
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

/// Summary statistics for charge points
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "total": 10,
    "online": 8,
    "offline": 2,
    "charging": 3
}))]
pub struct ChargePointStats {
    pub total: u32,
    pub online: u32,
    pub offline: u32,
    pub charging: u32,
}
