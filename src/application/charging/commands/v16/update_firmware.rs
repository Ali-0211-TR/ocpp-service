//! v1.6 UpdateFirmware command

use chrono::{DateTime, Utc};
use rust_ocpp::v1_6::messages::update_firmware::UpdateFirmwareRequest;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Send UpdateFirmware to a v1.6 charge point.
///
/// Note: v1.6 UpdateFirmwareResponse is an empty struct (no status),
/// so we return "Accepted" on success (the CP acknowledges the request).
pub async fn update_firmware(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    location: &str,
    retrieve_date: DateTime<Utc>,
    retries: Option<i32>,
    retry_interval: Option<i32>,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        location,
        %retrieve_date,
        "v1.6 UpdateFirmware"
    );

    let request = UpdateFirmwareRequest {
        location: location.to_string(),
        retries,
        retrieve_date,
        retry_interval,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    // v1.6 UpdateFirmware has an empty response — any successful call means accepted
    let _result = command_sender
        .send_command(charge_point_id, "UpdateFirmware", payload)
        .await?;

    Ok("Accepted".to_string())
}
