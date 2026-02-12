//! OCPP 1.6 Action handlers
//!
//! Routes OCPP 1.6 action names to their respective handlers.
//! Actions are dispatched by string name (parsed from `OcppFrame::Call`).
//! Payloads are deserialized into `rust_ocpp::v1_6` types within each handler.

use serde_json::Value;
use tracing::{error, warn};

use crate::application::OcppHandlerV16;

mod handle_authorize;
mod handle_boot_notification;
mod handle_data_transfer;
mod handle_diagnostics_status_notification;
mod handle_firmware_status_notification;
mod handle_heartbeat;
mod handle_meter_values;
mod handle_security_event_notification;
mod handle_start_transaction;
mod handle_status_notification;
mod handle_stop_transaction;

pub use handle_authorize::handle_authorize;
pub use handle_boot_notification::handle_boot_notification;
pub use handle_data_transfer::handle_data_transfer;
pub use handle_diagnostics_status_notification::handle_diagnostics_status_notification;
pub use handle_firmware_status_notification::handle_firmware_status_notification;
pub use handle_heartbeat::handle_heartbeat;
pub use handle_meter_values::handle_meter_values;
pub use handle_security_event_notification::handle_security_event_notification;
pub use handle_start_transaction::handle_start_transaction;
pub use handle_status_notification::handle_status_notification;
pub use handle_stop_transaction::handle_stop_transaction;

/// Routes OCPP 1.6 actions to their respective handlers.
///
/// `action` is the string action name from the OCPP-J Call frame.
/// `payload` is the raw JSON payload. Each handler deserializes it
/// into the appropriate `rust_ocpp::v1_6` request type.
///
/// Returns a `serde_json::Value` representing the response payload.
pub async fn v16_action_matcher(handler: &OcppHandlerV16, action: &str, payload: &Value) -> Value {
    match action {
        "Authorize" => handle_authorize(handler, payload).await,
        "BootNotification" => handle_boot_notification(handler, payload).await,
        "DataTransfer" => handle_data_transfer(handler, payload).await,
        "DiagnosticsStatusNotification" => {
            handle_diagnostics_status_notification(handler, payload).await
        }
        "FirmwareStatusNotification" => handle_firmware_status_notification(handler, payload).await,
        "Heartbeat" => handle_heartbeat(handler, payload).await,
        "MeterValues" => handle_meter_values(handler, payload).await,
        "SecurityEventNotification" => handle_security_event_notification(handler, payload).await,
        "StartTransaction" => handle_start_transaction(handler, payload).await,
        "StatusNotification" => handle_status_notification(handler, payload).await,
        "StopTransaction" => handle_stop_transaction(handler, payload).await,

        unknown => {
            // CS→CP actions or truly unknown actions
            if is_cs_to_cp_action(unknown) {
                warn!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    action = unknown,
                    "Received CS→CP action from charge point (protocol error)"
                );
            } else {
                error!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    action = unknown,
                    "Unknown OCPP 1.6 action"
                );
            }
            serde_json::json!({})
        }
    }
}

/// Check if the action is a CS→CP action (should never arrive from a CP).
fn is_cs_to_cp_action(action: &str) -> bool {
    matches!(
        action,
        "CancelReservation"
            | "ChangeAvailability"
            | "ChangeConfiguration"
            | "ClearCache"
            | "ClearChargingProfile"
            | "GetCompositeSchedule"
            | "GetConfiguration"
            | "GetDiagnostics"
            | "GetLocalListVersion"
            | "RemoteStartTransaction"
            | "RemoteStopTransaction"
            | "ReserveNow"
            | "Reset"
            | "SendLocalList"
            | "SetChargingProfile"
            | "TriggerMessage"
            | "UnlockConnector"
            | "UpdateFirmware"
    )
}
