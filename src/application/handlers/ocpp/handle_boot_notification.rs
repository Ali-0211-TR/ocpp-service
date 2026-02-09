//! BootNotification handler

use chrono::Utc;
use log::info;
use ocpp_rs::v16::call::BootNotification;
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::data_types::DateTimeWrapper;
use ocpp_rs::v16::enums::ParsedGenericStatus;

use crate::application::OcppHandler;
use crate::notifications::{BootNotificationEvent, Event};

pub async fn handle_boot_notification(
    handler: &OcppHandler,
    payload: BootNotification,
) -> ResultPayload {
    info!(
        "[{}] BootNotification - Vendor: {}, Model: {}",
        handler.charge_point_id, payload.charge_point_vendor, payload.charge_point_model
    );

    // Register or update charge point in storage
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

    // Auto-provision connector 0 (represents the whole station in OCPP 1.6)
    // and at least one physical connector if none exist yet.
    // Real connector set will be populated by StatusNotification messages.
    let _ = handler
        .service
        .ensure_connectors(&handler.charge_point_id, 1)
        .await;

    // Publish boot notification event
    handler.event_bus.publish(Event::BootNotification(BootNotificationEvent {
        charge_point_id: handler.charge_point_id.clone(),
        vendor: payload.charge_point_vendor.clone(),
        model: payload.charge_point_model.clone(),
        serial_number: payload.charge_point_serial_number.clone(),
        firmware_version: payload.firmware_version.clone(),
        timestamp: Utc::now(),
    }));

    ResultPayload::BootNotification(ocpp_rs::v16::call_result::BootNotification {
        current_time: DateTimeWrapper::new(Utc::now()),
        interval: 300, // Heartbeat interval in seconds
        status: ParsedGenericStatus::Accepted,
    })
}