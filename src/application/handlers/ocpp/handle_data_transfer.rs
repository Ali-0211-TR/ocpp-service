//! DataTransfer handler

use log::info;
use ocpp_rs::v16::call::DataTransfer;
use ocpp_rs::v16::call_result::{ResultPayload, StatusResponses};
use ocpp_rs::v16::enums::ParsedGenericStatus;

use crate::application::OcppHandler;

pub async fn handle_data_transfer(handler: &OcppHandler, payload: DataTransfer) -> ResultPayload {
    info!(
        "[{}] DataTransfer - VendorId: {}, MessageId: {:?}",
        handler.charge_point_id, payload.vendor_id, payload.message_id
    );

    // Handle vendor-specific data transfer
    // This can be extended based on vendor requirements

    ResultPayload::PossibleStatusResponse(StatusResponses::DataTransfer(
        ocpp_rs::v16::call_result::DataTransfer {
            status: ParsedGenericStatus::Accepted,
            data: None,
        },
    ))
}