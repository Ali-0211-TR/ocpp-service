//! FirmwareStatusNotification handler

use log::info;
use ocpp_rs::v16::call::FirmwareStatusNotification;
use ocpp_rs::v16::call_result::{EmptyResponse, EmptyResponses, ResultPayload};

use crate::application::OcppHandler;

pub async fn handle_firmware_status_notification(
    handler: &OcppHandler,
    payload: FirmwareStatusNotification,
) -> ResultPayload {
    info!(
        "[{}] FirmwareStatusNotification - Status: {:?}",
        handler.charge_point_id, payload.status
    );

    ResultPayload::PossibleEmptyResponse(EmptyResponses::EmptyResponse(EmptyResponse {}))
}