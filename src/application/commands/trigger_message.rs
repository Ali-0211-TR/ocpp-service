//! Trigger Message command

use rust_ocpp::v1_6::messages::trigger_message::{TriggerMessageRequest, TriggerMessageResponse};
use rust_ocpp::v1_6::types::MessageTrigger;
use tracing::info;

use super::{CommandError, SharedCommandSender};

#[derive(Debug, Clone, Copy)]
pub enum TriggerType {
    BootNotification,
    DiagnosticsStatusNotification,
    FirmwareStatusNotification,
    Heartbeat,
    MeterValues,
    StatusNotification,
}

impl TriggerType {
    fn to_message_trigger(self) -> MessageTrigger {
        match self {
            TriggerType::BootNotification => MessageTrigger::BootNotification,
            TriggerType::DiagnosticsStatusNotification => {
                MessageTrigger::DiagnosticsStatusNotification
            }
            TriggerType::FirmwareStatusNotification => MessageTrigger::FirmwareStatusNotification,
            TriggerType::Heartbeat => MessageTrigger::Heartbeat,
            TriggerType::MeterValues => MessageTrigger::MeterValues,
            TriggerType::StatusNotification => MessageTrigger::StatusNotification,
        }
    }
}

pub async fn trigger_message(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    requested_message: TriggerType,
    connector_id: Option<u32>,
) -> Result<String, CommandError> {
    info!(charge_point_id, ?requested_message, ?connector_id, "TriggerMessage");

    let request = TriggerMessageRequest {
        requested_message: requested_message.to_message_trigger(),
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
