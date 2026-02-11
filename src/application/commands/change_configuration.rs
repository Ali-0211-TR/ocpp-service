//! Change Configuration command

use ocpp_rs::v16::call::{Action, ChangeConfiguration};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::ParsedGenericStatus;
use tracing::info;

use super::{CommandError, SharedCommandSender};

pub async fn change_configuration(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    key: String,
    value: String,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(charge_point_id, key = key.as_str(), value = value.as_str(), "ChangeConfiguration");

    let action = Action::ChangeConfiguration(ChangeConfiguration { key, value });
    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(sr) => Ok(sr.get_status().clone()),
        ResultPayload::PossibleEmptyResponse(empty_response) => match empty_response {
            ocpp_rs::v16::call_result::EmptyResponses::EmptyResponse(_) => {
                Ok(ParsedGenericStatus::Accepted)
            }
            _ => Err(CommandError::InvalidResponse("Unexpected empty response type".to_string())),
        },
        _ => Err(CommandError::InvalidResponse("Unexpected response type".to_string())),
    }
}
