//! StatusNotification handler

use chrono::Utc;
use rust_ocpp::v1_6::messages::status_notification::{
    StatusNotificationRequest, StatusNotificationResponse,
};
use rust_ocpp::v1_6::types::ChargePointStatus;
use serde_json::Value;
use tracing::{error, info};

use crate::application::events::{ConnectorStatusChangedEvent, Event};
use crate::application::OcppHandlerV16;
use crate::domain::ConnectorStatus;

pub async fn handle_status_notification(handler: &OcppHandlerV16, payload: &Value) -> Value {
    let req: StatusNotificationRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(charge_point_id = handler.charge_point_id.as_str(), error = %e, "Failed to parse StatusNotification");
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        connector_id = req.connector_id,
        status = ?req.status,
        error_code = ?req.error_code,
        "StatusNotification"
    );

    let connector_status = match req.status {
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
        .update_connector_status(&handler.charge_point_id, req.connector_id, connector_status)
        .await;

    handler.event_bus.publish(Event::ConnectorStatusChanged(ConnectorStatusChangedEvent {
        charge_point_id: handler.charge_point_id.clone(),
        connector_id: req.connector_id,
        status: format!("{:?}", req.status),
        error_code: Some(format!("{:?}", req.error_code)),
        info: req.info.clone(),
        timestamp: req.timestamp.unwrap_or_else(Utc::now),
    }));

    serde_json::to_value(&StatusNotificationResponse {}).unwrap_or_default()
}
