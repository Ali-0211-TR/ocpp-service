//! BootNotification handler

use chrono::Utc;
use rust_ocpp::v1_6::messages::boot_notification::{
    BootNotificationRequest, BootNotificationResponse,
};
use rust_ocpp::v1_6::types::RegistrationStatus;
use tracing::{error, info};

use crate::application::events::{BootNotificationEvent, Event};
use crate::application::OcppHandlerV16;

pub async fn handle_boot_notification(
    handler: &OcppHandlerV16,
    payload: &serde_json::Value,
) -> serde_json::Value {
    let payload: BootNotificationRequest = match serde_json::from_value(payload.clone()) {
        Ok(p) => p,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "Failed to deserialize BootNotificationRequest"
            );
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        vendor = payload.charge_point_vendor.as_str(),
        model = payload.charge_point_model.as_str(),
        "BootNotification"
    );

    let _ = handler
        .service
        .register_or_update(
            &handler.charge_point_id,
            &payload.charge_point_vendor,
            &payload.charge_point_model,
            payload.charge_point_serial_number.as_deref(),
            payload.firmware_version.as_deref(),
        )
        .await;

    let _ = handler
        .service
        .ensure_connectors(&handler.charge_point_id, 1)
        .await;

    handler.event_bus.publish(Event::BootNotification(BootNotificationEvent {
        charge_point_id: handler.charge_point_id.clone(),
        vendor: payload.charge_point_vendor.clone(),
        model: payload.charge_point_model.clone(),
        serial_number: payload.charge_point_serial_number.clone(),
        firmware_version: payload.firmware_version.clone(),
        timestamp: Utc::now(),
    }));

    let response = BootNotificationResponse {
        current_time: Utc::now(),
        interval: 300,
        status: RegistrationStatus::Accepted,
    };

    serde_json::to_value(&response).unwrap_or_default()
}
