//! v2.0.1 GetTransactionStatus command

use rust_ocpp::v2_0_1::messages::get_transaction_status::{
    GetTransactionStatusRequest, GetTransactionStatusResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Result of a GetTransactionStatus command.
#[derive(Debug, Clone)]
pub struct GetTransactionStatusResult {
    /// Whether the transaction is still ongoing.
    pub ongoing_indicator: Option<bool>,
    /// Whether the station still has messages queued for delivery.
    pub messages_in_queue: bool,
}

pub async fn get_transaction_status(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    transaction_id: Option<String>,
) -> Result<GetTransactionStatusResult, CommandError> {
    info!(
        charge_point_id,
        transaction_id = ?transaction_id,
        "v2.0.1 GetTransactionStatus"
    );

    let request = GetTransactionStatusRequest { transaction_id };

    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "GetTransactionStatus", payload)
        .await?;

    let response: GetTransactionStatusResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(GetTransactionStatusResult {
        ongoing_indicator: response.ongoing_indicator,
        messages_in_queue: response.messages_in_queue,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_transaction_status_result() {
        let result = GetTransactionStatusResult {
            ongoing_indicator: Some(true),
            messages_in_queue: false,
        };
        assert_eq!(result.ongoing_indicator, Some(true));
        assert!(!result.messages_in_queue);
    }
}
