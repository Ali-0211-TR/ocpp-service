//! v2.0.1 GetLog command

use chrono::{DateTime, Utc};
use rust_ocpp::v2_0_1::datatypes::log_parameters_type::LogParametersType;
use rust_ocpp::v2_0_1::enumerations::log_enum_type::LogEnumType;
use rust_ocpp::v2_0_1::messages::get_log::{GetLogRequest, GetLogResponse};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Result of a GetLog command.
pub struct GetLogResult {
    pub status: String,
    pub filename: Option<String>,
}

pub async fn get_log(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    log_type: &str,
    location: &str,
    request_id: i32,
    retries: Option<i32>,
    retry_interval: Option<i32>,
    oldest_timestamp: Option<DateTime<Utc>>,
    latest_timestamp: Option<DateTime<Utc>>,
) -> Result<GetLogResult, CommandError> {
    info!(
        charge_point_id,
        log_type,
        location,
        request_id,
        "v2.0.1 GetLog"
    );

    let log_type_enum = match log_type {
        "SecurityLog" => LogEnumType::SecurityLog,
        _ => LogEnumType::DiagnosticsLog,
    };

    let request = GetLogRequest {
        log_type: log_type_enum,
        request_id,
        retries,
        retry_interval,
        log: LogParametersType {
            remote_location: location.to_string(),
            oldest_timestamp,
            latest_timestamp,
        },
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "GetLog", payload)
        .await?;

    let response: GetLogResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(GetLogResult {
        status: format!("{:?}", response.status),
        filename: response.filename,
    })
}
