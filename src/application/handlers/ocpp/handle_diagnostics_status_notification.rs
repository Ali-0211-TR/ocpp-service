//! DiagnosticsStatusNotification handler

use ocpp_rs::v16::call::DiagnosticsStatusNotification;
use ocpp_rs::v16::call_result::{EmptyResponse, EmptyResponses, ResultPayload};
use tracing::info;

use crate::application::OcppHandler;

pub async fn handle_diagnostics_status_notification(
    handler: &OcppHandler,
    payload: DiagnosticsStatusNotification,
) -> ResultPayload {
    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        status = ?payload.status,
        "DiagnosticsStatusNotification"
    );

    ResultPayload::PossibleEmptyResponse(EmptyResponses::EmptyResponse(EmptyResponse {}))
}
