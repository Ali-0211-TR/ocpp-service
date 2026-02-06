//! SecurityEventNotification handler

use log::info;
use ocpp_rs::v16::call::SecurityEventNotification;
use ocpp_rs::v16::call_result::{EmptyResponse, EmptyResponses, ResultPayload};

use crate::application::OcppHandler;

pub async fn handle_security_event_notification(
    handler: &OcppHandler,
    payload: SecurityEventNotification,
) -> ResultPayload {
    info!(
        "[{}] SecurityEventNotification - Type: {}, Timestamp: {:?}",
        handler.charge_point_id, payload.event_type, payload.timestamp
    );

    // TODO: Log security events for monitoring/alerting

    ResultPayload::PossibleEmptyResponse(EmptyResponses::EmptyResponse(EmptyResponse {}))
}