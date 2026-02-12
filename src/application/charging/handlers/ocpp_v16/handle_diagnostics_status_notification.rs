//! DiagnosticsStatusNotification handler

use rust_ocpp::v1_6::messages::diagnostics_status_notification::{
    DiagnosticsStatusNotificationRequest, DiagnosticsStatusNotificationResponse,
};
use serde_json::Value;
use tracing::{error, info};

use crate::application::OcppHandlerV16;

pub async fn handle_diagnostics_status_notification(
    handler: &OcppHandlerV16,
    payload: &Value,
) -> Value {
    let req: DiagnosticsStatusNotificationRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(charge_point_id = handler.charge_point_id.as_str(), error = %e, "Failed to parse DiagnosticsStatusNotification");
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        status = ?req.status,
        "DiagnosticsStatusNotification"
    );

    serde_json::to_value(&DiagnosticsStatusNotificationResponse {}).unwrap_or_default()
}
