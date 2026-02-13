//! v2.0.1 UnlockConnector command

use rust_ocpp::v2_0_1::messages::unlock_connector::{
    UnlockConnectorRequest, UnlockConnectorResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// In v2.0.1 UnlockConnector requires both `evse_id` and `connector_id`.
pub async fn unlock_connector(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    evse_id: i32,
    connector_id: i32,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        evse_id, connector_id, "v2.0.1 UnlockConnector"
    );

    let request = UnlockConnectorRequest {
        evse_id,
        connector_id,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "UnlockConnector", payload)
        .await?;

    let response: UnlockConnectorResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
