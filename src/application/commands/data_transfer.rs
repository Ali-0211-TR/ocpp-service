//! Data Transfer command

use log::info;
use ocpp_rs::v16::call::{Action, DataTransfer};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::ParsedGenericStatus;

use super::{CommandError, SharedCommandSender};

/// Data transfer result
#[derive(Debug)]
pub struct DataTransferResult {
    /// Status: Accepted, Rejected, UnknownMessageId, UnknownVendorId
    pub status: ParsedGenericStatus,
    /// Optional data returned from the charge point
    pub data: Option<String>,
}

/// Send vendor-specific data to a charge point
///
/// Used for proprietary extensions to the OCPP protocol
pub async fn data_transfer(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    vendor_id: String,
    message_id: Option<String>,
    data: Option<String>,
) -> Result<DataTransferResult, CommandError> {
    info!(
        "[{}] DataTransfer - VendorId: {}, MessageId: {:?}",
        charge_point_id, vendor_id, message_id
    );

    let action = Action::DataTransfer(DataTransfer {
        vendor_id,
        message_id,
        data,
    });

    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(status_response) => {
            match status_response {
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
                _ => Err(CommandError::InvalidResponse(
                    "Unexpected status response type".to_string(),
                )),
            }
        }
        _ => Err(CommandError::InvalidResponse(
            "Unexpected response type".to_string(),
        )),
    }
}
