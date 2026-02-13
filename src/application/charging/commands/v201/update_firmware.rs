//! v2.0.1 UpdateFirmware command

use chrono::{DateTime, Utc};
use rust_ocpp::v2_0_1::datatypes::firmware_type::FirmwareType;
use rust_ocpp::v2_0_1::messages::update_firmware::{UpdateFirmwareRequest, UpdateFirmwareResponse};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

pub async fn update_firmware(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    location: &str,
    retrieve_date: DateTime<Utc>,
    request_id: i32,
    retries: Option<i32>,
    retry_interval: Option<i32>,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        location,
        request_id,
        %retrieve_date,
        "v2.0.1 UpdateFirmware"
    );

    let request = UpdateFirmwareRequest {
        retries,
        retry_interval,
        request_id,
        firmware: FirmwareType {
            location: location.to_string(),
            retrieve_date_time: retrieve_date,
            install_date_time: None,
            signing_certificate: None,
            signature: None,
        },
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "UpdateFirmware", payload)
        .await?;

    let response: UpdateFirmwareResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
