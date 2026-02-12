//! V201 FirmwareStatusNotification handler

use rust_ocpp::v2_0_1::messages::firmware_status_notification::{
    FirmwareStatusNotificationRequest, FirmwareStatusNotificationResponse,
};
use serde_json::Value;
use tracing::{error, info};

use crate::application::OcppHandlerV201;

pub async fn handle_firmware_status_notification(
    handler: &OcppHandlerV201,
    payload: &Value,
) -> Value {
    let req: FirmwareStatusNotificationRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse FirmwareStatusNotification"
            );
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        status = ?req.status,
        request_id = ?req.request_id,
        "V201 FirmwareStatusNotification"
    );

    serde_json::to_value(&FirmwareStatusNotificationResponse {}).unwrap_or_default()
}
