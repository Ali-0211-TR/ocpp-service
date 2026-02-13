//! v2.0.1 DataTransfer command

use rust_ocpp::v2_0_1::messages::datatransfer::{DataTransferRequest, DataTransferResponse};
use tracing::info;

use crate::application::charging::commands::{CommandError, DataTransferResult, SharedCommandSender};

pub async fn data_transfer(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    vendor_id: String,
    message_id: Option<String>,
    data: Option<String>,
) -> Result<DataTransferResult, CommandError> {
    info!(
        charge_point_id,
        vendor_id = vendor_id.as_str(),
        ?message_id,
        "v2.0.1 DataTransfer"
    );

    let request = DataTransferRequest {
        vendor_id,
        message_id,
        data,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "DataTransfer", payload)
        .await?;

    let response: DataTransferResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(DataTransferResult {
        status: format!("{:?}", response.status),
        data: response.data,
    })
}
