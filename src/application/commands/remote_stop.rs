//! Remote Stop Transaction command

use rust_ocpp::v1_6::messages::remote_stop_transaction::{
    RemoteStopTransactionRequest, RemoteStopTransactionResponse,
};
use tracing::info;

use super::{CommandError, SharedCommandSender};

pub async fn remote_stop_transaction(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    transaction_id: i32,
) -> Result<String, CommandError> {
    info!(charge_point_id, transaction_id, "RemoteStopTransaction");

    let request = RemoteStopTransactionRequest { transaction_id };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "RemoteStopTransaction", payload)
        .await?;

    let response: RemoteStopTransactionResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
