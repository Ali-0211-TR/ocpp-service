//! V201 StatusNotification handler
//!
//! In OCPP 2.0.1, StatusNotification carries `connector_status`, `evse_id`,
//! and `connector_id` (instead of the flat connector_id + ChargePointStatus).

use rust_ocpp::v2_0_1::enumerations::connector_status_enum_type::ConnectorStatusEnumType;
use rust_ocpp::v2_0_1::messages::status_notification::{
    StatusNotificationRequest, StatusNotificationResponse,
};
use serde_json::Value;
use tracing::{error, info};

use crate::application::events::{ConnectorStatusChangedEvent, Event};
use crate::application::handlers::OcppHandlerV201;
use crate::domain::ConnectorStatus;

pub async fn handle_status_notification(handler: &OcppHandlerV201, payload: &Value) -> Value {
    let req: StatusNotificationRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse StatusNotification"
            );
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        evse_id = req.evse_id,
        connector_id = req.connector_id,
        status = ?req.connector_status,
        "V201 StatusNotification"
    );

    // Map V201 ConnectorStatusEnumType → domain ConnectorStatus
    // V201 has fewer states: Available, Occupied, Reserved, Unavailable, Faulted
    let connector_status = match req.connector_status {
        ConnectorStatusEnumType::Available => ConnectorStatus::Available,
        ConnectorStatusEnumType::Occupied => ConnectorStatus::Charging,
        ConnectorStatusEnumType::Reserved => ConnectorStatus::Reserved,
        ConnectorStatusEnumType::Unavailable => ConnectorStatus::Unavailable,
        ConnectorStatusEnumType::Faulted => ConnectorStatus::Faulted,
    };

    // In V201, evse_id maps to connector_id in our domain model.
    // If evse_id == 0, it refers to the whole station.
    let domain_connector_id = req.evse_id as u32;

    let _ = handler
        .service
        .update_connector_status(
            &handler.charge_point_id,
            domain_connector_id,
            connector_status,
        )
        .await;

    handler
        .event_bus
        .publish(Event::ConnectorStatusChanged(ConnectorStatusChangedEvent {
            charge_point_id: handler.charge_point_id.clone(),
            connector_id: domain_connector_id,
            status: format!("{:?}", req.connector_status),
            error_code: None,
            info: None,
            timestamp: req.timestamp,
        }));

    serde_json::to_value(&StatusNotificationResponse {}).unwrap_or_default()
}
