//! SecurityEventNotification handler

use ocpp_rs::v16::call::SecurityEventNotification;
use ocpp_rs::v16::call_result::{EmptyResponse, EmptyResponses, ResultPayload};
use tracing::info;

use crate::application::OcppHandler;

pub async fn handle_security_event_notification(
    handler: &OcppHandler,
    payload: SecurityEventNotification,
) -> ResultPayload {
    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        event_type = payload.event_type.as_str(),
        timestamp = ?payload.timestamp,
        "SecurityEventNotification"
    );

    ResultPayload::PossibleEmptyResponse(EmptyResponses::EmptyResponse(EmptyResponse {}))
}
