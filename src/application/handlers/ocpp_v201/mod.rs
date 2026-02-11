//! OCPP 2.0.1 Action handlers
//!
//! Routes OCPP 2.0.1 action names to their respective handlers.
//! Actions are dispatched by string name (parsed from `OcppFrame::Call`).
//! Payloads are deserialized into `rust_ocpp::v2_0_1` types within each handler.

use serde_json::Value;
use tracing::{error, warn};

use crate::application::handlers::OcppHandlerV201;

mod handle_authorize;
mod handle_boot_notification;
mod handle_data_transfer;
mod handle_firmware_status_notification;
mod handle_heartbeat;
mod handle_meter_values;
mod handle_security_event_notification;
mod handle_status_notification;
mod handle_transaction_event;

pub use handle_authorize::handle_authorize;
pub use handle_boot_notification::handle_boot_notification;
pub use handle_data_transfer::handle_data_transfer;
pub use handle_firmware_status_notification::handle_firmware_status_notification;
pub use handle_heartbeat::handle_heartbeat;
pub use handle_meter_values::handle_meter_values;
pub use handle_security_event_notification::handle_security_event_notification;
pub use handle_status_notification::handle_status_notification;
pub use handle_transaction_event::handle_transaction_event;

/// Routes OCPP 2.0.1 actions to their respective handlers.
///
/// `action` is the string action name from the OCPP-J Call frame.
/// `payload` is the raw JSON payload. Each handler deserializes it
/// into the appropriate `rust_ocpp::v2_0_1` request type.
///
/// Returns a `serde_json::Value` representing the response payload.
pub async fn action_matcher(handler: &OcppHandlerV201, action: &str, payload: &Value) -> Value {
    match action {
        "Authorize" => handle_authorize(handler, payload).await,
        "BootNotification" => handle_boot_notification(handler, payload).await,
        "DataTransfer" => handle_data_transfer(handler, payload).await,
        "FirmwareStatusNotification" => {
            handle_firmware_status_notification(handler, payload).await
        }
        "Heartbeat" => handle_heartbeat(handler, payload).await,
        "MeterValues" => handle_meter_values(handler, payload).await,
        "SecurityEventNotification" => {
            handle_security_event_notification(handler, payload).await
        }
        "StatusNotification" => handle_status_notification(handler, payload).await,
        "TransactionEvent" => handle_transaction_event(handler, payload).await,

        unknown => {
            if is_csms_to_cs_action(unknown) {
                warn!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    action = unknown,
                    "V201: Received CSMS→CS action from charging station (protocol error)"
                );
            } else {
                error!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    action = unknown,
                    "Unknown OCPP 2.0.1 action"
                );
            }
            serde_json::json!({})
        }
    }
}

/// Check if the action is a CSMS→CS action (should never arrive from a CS).
fn is_csms_to_cs_action(action: &str) -> bool {
    matches!(
        action,
        "CancelReservation"
            | "CertificateSigned"
            | "ChangeAvailability"
            | "ClearCache"
            | "ClearChargingProfile"
            | "ClearDisplayMessage"
            | "ClearVariableMonitoring"
            | "CostUpdated"
            | "CustomerInformation"
            | "DeleteCertificate"
            | "GetBaseReport"
            | "GetChargingProfiles"
            | "GetCompositeSchedule"
            | "GetDisplayMessages"
            | "GetInstalledCertificateIds"
            | "GetLocalListVersion"
            | "GetLog"
            | "GetMonitoringReport"
            | "GetReport"
            | "GetTransactionStatus"
            | "GetVariables"
            | "InstallCertificate"
            | "PublishFirmware"
            | "RequestStartTransaction"
            | "RequestStopTransaction"
            | "ReserveNow"
            | "Reset"
            | "SendLocalList"
            | "SetChargingProfile"
            | "SetDisplayMessage"
            | "SetMonitoringBase"
            | "SetMonitoringLevel"
            | "SetNetworkProfile"
            | "SetVariableMonitoring"
            | "SetVariables"
            | "TriggerMessage"
            | "UnlockConnector"
            | "UnpublishFirmware"
            | "UpdateFirmware"
    )
}
