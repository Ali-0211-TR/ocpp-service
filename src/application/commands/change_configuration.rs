//! Change Configuration command

use rust_ocpp::v1_6::messages::change_configuration::{
    ChangeConfigurationRequest, ChangeConfigurationResponse,
};
use tracing::info;

use super::{CommandError, SharedCommandSender};

pub async fn change_configuration(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    key: String,
    value: String,
) -> Result<String, CommandError> {
    info!(charge_point_id, key = key.as_str(), value = value.as_str(), "ChangeConfiguration");

    let request = ChangeConfigurationRequest { key, value };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "ChangeConfiguration", payload)
        .await?;

    let response: ChangeConfigurationResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
