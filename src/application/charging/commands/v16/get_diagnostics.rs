//! v1.6 GetDiagnostics command

use chrono::{DateTime, Utc};
use rust_ocpp::v1_6::messages::get_diagnostics::{GetDiagnosticsRequest, GetDiagnosticsResponse};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Send GetDiagnostics to a v1.6 charge point.
///
/// Returns the filename where diagnostics will be uploaded, or None.
pub async fn get_diagnostics(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    location: &str,
    retries: Option<i32>,
    retry_interval: Option<i32>,
    start_time: Option<DateTime<Utc>>,
    stop_time: Option<DateTime<Utc>>,
) -> Result<Option<String>, CommandError> {
    info!(
        charge_point_id,
        location,
        "v1.6 GetDiagnostics"
    );

    let request = GetDiagnosticsRequest {
        location: location.to_string(),
        retries,
        retry_interval,
        start_time,
        stop_time,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "GetDiagnostics", payload)
        .await?;

    let response: GetDiagnosticsResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(response.file_name)
}
