//! v2.0.1 ClearCache command

use rust_ocpp::v2_0_1::messages::clear_cache::{ClearCacheRequest, ClearCacheResponse};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

pub async fn clear_cache(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
) -> Result<String, CommandError> {
    info!(charge_point_id, "v2.0.1 ClearCache");

    let request = ClearCacheRequest {};
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "ClearCache", payload)
        .await?;

    let response: ClearCacheResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
