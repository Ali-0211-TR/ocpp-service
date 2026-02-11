//! Data Transfer command

use ocpp_rs::v16::call::{Action, DataTransfer};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::ParsedGenericStatus;
use tracing::info;

use super::{CommandError, SharedCommandSender};

#[derive(Debug)]
pub struct DataTransferResult {
    pub status: ParsedGenericStatus,
    pub data: Option<String>,
}

pub async fn data_transfer(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    vendor_id: String,
    message_id: Option<String>,
    data: Option<String>,
) -> Result<DataTransferResult, CommandError> {
    info!(charge_point_id, vendor_id = vendor_id.as_str(), ?message_id, "DataTransfer");

    let action = Action::DataTransfer(DataTransfer {
        vendor_id,
        message_id,
        data,
    });
    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(status_response) => match status_response {
            ocpp_rs::v16::call_result::StatusResponses::DataTransfer(dt) => {
                Ok(DataTransferResult {
                    status: dt.status,
                    data: dt.data,
                })
            }
            ocpp_rs::v16::call_result::StatusResponses::StatusResponse(sr) => {
                Ok(DataTransferResult {
                    status: sr.status,
                    data: None,
                })
            }
            _ => Err(CommandError::InvalidResponse("Unexpected status response type".to_string())),
        },
        _ => Err(CommandError::InvalidResponse("Unexpected response type".to_string())),
    }
}
