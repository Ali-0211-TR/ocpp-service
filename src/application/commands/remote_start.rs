//! Remote Start Transaction command

use rust_ocpp::v1_6::messages::remote_start_transaction::{
    RemoteStartTransactionRequest, RemoteStartTransactionResponse,
};
use tracing::info;

use super::{CommandError, SharedCommandSender};

pub async fn remote_start_transaction(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    id_tag: &str,
    connector_id: Option<u32>,
) -> Result<String, CommandError> {
    info!(charge_point_id, id_tag, ?connector_id, "RemoteStartTransaction");

    let request = RemoteStartTransactionRequest {
        id_tag: id_tag.to_string(),
        connector_id,
        charging_profile: None,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "RemoteStartTransaction", payload)
        .await?;

    let response: RemoteStartTransactionResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
