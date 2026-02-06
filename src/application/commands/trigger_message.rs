//! Trigger Message command

use log::info;
use ocpp_rs::v16::call::{Action, TriggerMessage};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::{MessageTrigger, ParsedGenericStatus};

use super::{CommandError, SharedCommandSender};

/// Message type to trigger
#[derive(Debug, Clone, Copy)]
pub enum TriggerType {
    BootNotification,
    DiagnosticsStatusNotification,
    FirmwareStatusNotification,
    Heartbeat,
    MeterValues,
    StatusNotification,
}

impl From<TriggerType> for MessageTrigger {
    fn from(trigger: TriggerType) -> Self {
        match trigger {
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

/// Trigger a message from the charge point
pub async fn trigger_message(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    requested_message: TriggerType,
    connector_id: Option<u32>,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(
        "[{}] TriggerMessage - Message: {:?}, ConnectorId: {:?}",
        charge_point_id, requested_message, connector_id
    );

    let action = Action::TriggerMessage(TriggerMessage {
        requested_message: requested_message.into(),
        connector_id,
    });

    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(status_response) => {
            Ok(status_response.get_status().clone())
        }
        _ => Err(CommandError::InvalidResponse(
            "Unexpected response type".to_string(),
        )),
    }
}
