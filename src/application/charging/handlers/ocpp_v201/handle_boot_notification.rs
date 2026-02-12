//! V201 BootNotification handler

use chrono::Utc;
use rust_ocpp::v2_0_1::enumerations::registration_status_enum_type::RegistrationStatusEnumType;
use rust_ocpp::v2_0_1::messages::boot_notification::{
    BootNotificationRequest, BootNotificationResponse,
};
use tracing::{error, info};

use crate::application::events::{BootNotificationEvent, Event};
use crate::application::OcppHandlerV201;
use crate::domain::OcppVersion;

pub async fn handle_boot_notification(
    handler: &OcppHandlerV201,
    payload: &serde_json::Value,
) -> serde_json::Value {
    // Some charging stations omit the mandatory `reason` field.
    // Inject a default ("PowerUp") before deserializing so we don't reject the message.
    let mut patched = payload.clone();
    if let Some(obj) = patched.as_object_mut() {
        obj.entry("reason").or_insert(serde_json::json!("PowerUp"));
    }

    let req: BootNotificationRequest = match serde_json::from_value(patched) {
        Ok(p) => p,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to deserialize BootNotificationRequest"
            );
            return serde_json::json!({});
        }
    };

    let cs = &req.charging_station;
    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        vendor = cs.vendor_name.as_str(),
        model = cs.model.as_str(),
        reason = ?req.reason,
        "V201 BootNotification"
    );

    // OCPP 2.0.1: modem ICCID/IMSI live inside charging_station.modem
    let (iccid, imsi) = cs
        .modem
        .as_ref()
        .map(|m| (m.iccid.as_deref(), m.imsi.as_deref()))
        .unwrap_or((None, None));

    let _ = handler
        .service
        .register_or_update(
            &handler.charge_point_id,
            &cs.vendor_name,
            &cs.model,
            cs.serial_number.as_deref(),
            cs.firmware_version.as_deref(),
            OcppVersion::V201,
            iccid,
            imsi,
            None, // meter_type: not in OCPP 2.0.1
            None, // meter_serial_number: not in OCPP 2.0.1
        )
        .await;

    let _ = handler
        .service
        .ensure_connectors(&handler.charge_point_id, 1)
        .await;

    handler
        .event_bus
        .publish(Event::BootNotification(BootNotificationEvent {
            charge_point_id: handler.charge_point_id.clone(),
            vendor: cs.vendor_name.clone(),
            model: cs.model.clone(),
            serial_number: cs.serial_number.clone(),
            firmware_version: cs.firmware_version.clone(),
            ocpp_version: OcppVersion::V201.version_string().to_string(),
            timestamp: Utc::now(),
        }));

    let response = BootNotificationResponse {
        current_time: Utc::now(),
        interval: 300,
        status: RegistrationStatusEnumType::Accepted,
        status_info: None,
    };

    serde_json::to_value(&response).unwrap_or_default()
}
