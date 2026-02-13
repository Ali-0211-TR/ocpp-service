//! v1.6 Trigger Message command

use rust_ocpp::v1_6::messages::trigger_message::{TriggerMessageRequest, TriggerMessageResponse};
use rust_ocpp::v1_6::types::MessageTrigger;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender, TriggerType};

pub async fn trigger_message(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    requested_message: TriggerType,
    connector_id: Option<u32>,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        ?requested_message,
        ?connector_id,
        "v1.6 TriggerMessage"
    );

    let trigger = match requested_message {
        TriggerType::BootNotification => MessageTrigger::BootNotification,
        TriggerType::DiagnosticsStatusNotification => {
            MessageTrigger::DiagnosticsStatusNotification
        }
        TriggerType::FirmwareStatusNotification => MessageTrigger::FirmwareStatusNotification,
        TriggerType::Heartbeat => MessageTrigger::Heartbeat,
        TriggerType::MeterValues => MessageTrigger::MeterValues,
        TriggerType::StatusNotification => MessageTrigger::StatusNotification,
    };

    let request = TriggerMessageRequest {
        requested_message: trigger,
        connector_id,
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
