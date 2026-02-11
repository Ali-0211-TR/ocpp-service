//! DataTransfer handler

use ocpp_rs::v16::call::DataTransfer;
use ocpp_rs::v16::call_result::{ResultPayload, StatusResponses};
use ocpp_rs::v16::enums::ParsedGenericStatus;
use tracing::info;

use crate::application::OcppHandler;

pub async fn handle_data_transfer(handler: &OcppHandler, payload: DataTransfer) -> ResultPayload {
    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        vendor_id = payload.vendor_id.as_str(),
        message_id = ?payload.message_id,
        "DataTransfer"
    );

    ResultPayload::PossibleStatusResponse(StatusResponses::DataTransfer(
        ocpp_rs::v16::call_result::DataTransfer {
            status: ParsedGenericStatus::Accepted,
            data: None,
        },
    ))
}
