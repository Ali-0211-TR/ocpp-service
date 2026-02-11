//! V201 SecurityEventNotification handler
//!
//! Unlike V1.6, SecurityEventNotification is a first-class message in OCPP 2.0.1.

use rust_ocpp::v2_0_1::messages::security_event_notification::{
    SecurityEventNotificationRequest, SecurityEventNotificationResponse,
};
use serde_json::Value;
use tracing::{error, info};

use crate::application::handlers::OcppHandlerV201;

pub async fn handle_security_event_notification(
    handler: &OcppHandlerV201,
    payload: &Value,
) -> Value {
    let req: SecurityEventNotificationRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse SecurityEventNotification"
            );
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        event_type = req.kind.as_str(),
        timestamp = %req.timestamp,
        tech_info = ?req.tech_info,
        "V201 SecurityEventNotification"
    );

    serde_json::to_value(&SecurityEventNotificationResponse {}).unwrap_or_default()
}
