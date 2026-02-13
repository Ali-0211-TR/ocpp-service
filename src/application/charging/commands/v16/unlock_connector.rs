//! v1.6 Unlock Connector command

use rust_ocpp::v1_6::messages::unlock_connector::{
    UnlockConnectorRequest, UnlockConnectorResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

pub async fn unlock_connector(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    connector_id: u32,
) -> Result<String, CommandError> {
    info!(charge_point_id, connector_id, "v1.6 UnlockConnector");

    let request = UnlockConnectorRequest { connector_id };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "UnlockConnector", payload)
        .await?;

    let response: UnlockConnectorResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
