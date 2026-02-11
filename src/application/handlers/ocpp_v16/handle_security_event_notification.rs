//! SecurityEventNotification handler
//!
//! SecurityEventNotification is NOT part of the official OCPP 1.6 standard,
//! but some charge points send it as a vendor extension.
//! We define a local deserialization struct since rust-ocpp v1_6 does not include it.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use tracing::{error, info};

use crate::application::OcppHandlerV16;

/// Local struct for SecurityEventNotification (vendor extension in OCPP 1.6)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecurityEventNotificationRequest {
    #[serde(rename = "type")]
    event_type: String,
    timestamp: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    tech_info: Option<String>,
}

pub async fn handle_security_event_notification(
    handler: &OcppHandlerV16,
    payload: &Value,
) -> Value {
    let req: SecurityEventNotificationRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(charge_point_id = handler.charge_point_id.as_str(), error = %e, "Failed to parse SecurityEventNotification");
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        event_type = req.event_type.as_str(),
        timestamp = ?req.timestamp,
        "SecurityEventNotification"
    );

    serde_json::json!({})
}
