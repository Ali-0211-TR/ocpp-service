//! FirmwareStatusNotification handler

use ocpp_rs::v16::call::FirmwareStatusNotification;
use ocpp_rs::v16::call_result::{EmptyResponse, EmptyResponses, ResultPayload};
use tracing::info;

use crate::application::OcppHandler;

pub async fn handle_firmware_status_notification(
    handler: &OcppHandler,
    payload: FirmwareStatusNotification,
) -> ResultPayload {
    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        status = ?payload.status,
        "FirmwareStatusNotification"
    );

    ResultPayload::PossibleEmptyResponse(EmptyResponses::EmptyResponse(EmptyResponse {}))
}
