//! Data Transfer command

use rust_ocpp::v1_6::messages::data_transfer::{DataTransferRequest, DataTransferResponse};
use tracing::info;

use super::{CommandError, SharedCommandSender};

#[derive(Debug)]
pub struct DataTransferResult {
    pub status: String,
    pub data: Option<String>,
}

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
        "DataTransfer"
    );

    let request = DataTransferRequest {
        vendor_string: vendor_id,
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
