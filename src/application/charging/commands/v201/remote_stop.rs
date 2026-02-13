//! v2.0.1 RequestStopTransaction command

use rust_ocpp::v2_0_1::messages::request_stop_transaction::{
    RequestStopTransactionRequest, RequestStopTransactionResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// In v2.0.1 transaction_id is a String, unlike v1.6 where it is i32.
/// The dispatcher converts the i32 to String before calling this function.
pub async fn remote_stop_transaction(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    transaction_id: &str,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        transaction_id, "v2.0.1 RequestStopTransaction"
    );

    let request = RequestStopTransactionRequest {
        transaction_id: transaction_id.to_string(),
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "RequestStopTransaction", payload)
        .await?;

    let response: RequestStopTransactionResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
