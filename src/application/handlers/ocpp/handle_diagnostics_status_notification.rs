//! DiagnosticsStatusNotification handler

use log::info;
use ocpp_rs::v16::call::DiagnosticsStatusNotification;
use ocpp_rs::v16::call_result::{EmptyResponse, EmptyResponses, ResultPayload};

use crate::application::OcppHandler;

pub async fn handle_diagnostics_status_notification(
    handler: &OcppHandler,
    payload: DiagnosticsStatusNotification,
) -> ResultPayload {
    info!(
        "[{}] DiagnosticsStatusNotification - Status: {:?}",
        handler.charge_point_id, payload.status
    );

    ResultPayload::PossibleEmptyResponse(EmptyResponses::EmptyResponse(EmptyResponse {}))
}