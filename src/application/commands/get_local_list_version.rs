//! Get Local List Version command

use rust_ocpp::v1_6::messages::get_local_list_version::{
    GetLocalListVersionRequest, GetLocalListVersionResponse,
};
use tracing::info;

use super::{CommandError, SharedCommandSender};

pub async fn get_local_list_version(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
) -> Result<i32, CommandError> {
    info!(charge_point_id, "GetLocalListVersion");

    let request = GetLocalListVersionRequest {};
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "GetLocalListVersion", payload)
        .await?;

    let response: GetLocalListVersionResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(response.list_version)
}
