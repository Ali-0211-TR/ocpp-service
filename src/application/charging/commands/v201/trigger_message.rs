//! v2.0.1 TriggerMessage command

use rust_ocpp::v2_0_1::messages::trigger_message::{
    TriggerMessageRequest, TriggerMessageResponse,
};
use rust_ocpp::v2_0_1::datatypes::evse_type::EVSEType;
use rust_ocpp::v2_0_1::enumerations::message_trigger_enum_type::MessageTriggerEnumType;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender, TriggerType};

pub async fn trigger_message(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    requested_message: TriggerType,
    evse_id: Option<i32>,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        ?requested_message,
        ?evse_id,
        "v2.0.1 TriggerMessage"
    );

    let trigger = match requested_message {
        TriggerType::BootNotification => MessageTriggerEnumType::BootNotification,
        TriggerType::Heartbeat => MessageTriggerEnumType::Heartbeat,
        TriggerType::MeterValues => MessageTriggerEnumType::MeterValues,
        TriggerType::StatusNotification => MessageTriggerEnumType::StatusNotification,
        TriggerType::FirmwareStatusNotification => {
            MessageTriggerEnumType::FirmwareStatusNotification
        }
        // DiagnosticsStatusNotification doesn't exist in v2.0.1 — use LogStatusNotification
        TriggerType::DiagnosticsStatusNotification => {
            MessageTriggerEnumType::LogStatusNotification
        }
    };

    let evse = evse_id.map(|id| EVSEType {
        id,
        connector_id: None,
    });

    let request = TriggerMessageRequest {
        requested_message: trigger,
        evse,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "TriggerMessage", payload)
        .await?;

    let response: TriggerMessageResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
