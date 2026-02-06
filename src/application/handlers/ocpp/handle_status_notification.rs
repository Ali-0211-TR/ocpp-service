//! StatusNotification handler

use chrono::Utc;
use log::info;
use ocpp_rs::v16::call::StatusNotification;
use ocpp_rs::v16::call_result::{EmptyResponse, EmptyResponses, ResultPayload};
use ocpp_rs::v16::enums::ChargePointStatus;

use crate::application::OcppHandler;
use crate::domain::ConnectorStatus;
use crate::notifications::{ConnectorStatusChangedEvent, Event};

pub async fn handle_status_notification(
    handler: &OcppHandler,
    payload: StatusNotification,
) -> ResultPayload {
    info!(
        "[{}] StatusNotification - Connector: {}, Status: {:?}, ErrorCode: {:?}",
        handler.charge_point_id, payload.connector_id, payload.status, payload.error_code
    );

    // Convert OCPP status to domain status
    let connector_status = match payload.status {
        ChargePointStatus::Available => ConnectorStatus::Available,
        ChargePointStatus::Preparing => ConnectorStatus::Preparing,
        ChargePointStatus::Charging => ConnectorStatus::Charging,
        ChargePointStatus::SuspendedEV => ConnectorStatus::SuspendedEV,
        ChargePointStatus::SuspendedEVSE => ConnectorStatus::SuspendedEVSE,
        ChargePointStatus::Finishing => ConnectorStatus::Finishing,
        ChargePointStatus::Reserved => ConnectorStatus::Reserved,
        ChargePointStatus::Unavailable => ConnectorStatus::Unavailable,
        ChargePointStatus::Faulted => ConnectorStatus::Faulted,
    };

    let _ = handler
        .service
        .update_connector_status(&handler.charge_point_id, payload.connector_id, connector_status)
        .await;

    // Publish connector status changed event
    handler.event_bus.publish(Event::ConnectorStatusChanged(ConnectorStatusChangedEvent {
        charge_point_id: handler.charge_point_id.clone(),
        connector_id: payload.connector_id,
        status: format!("{:?}", payload.status),
        error_code: Some(format!("{:?}", payload.error_code)),
        info: payload.info.clone(),
        timestamp: payload.timestamp.map(|t| t.inner()).unwrap_or_else(Utc::now),
    }));

    ResultPayload::PossibleEmptyResponse(EmptyResponses::EmptyResponse(EmptyResponse {}))
}