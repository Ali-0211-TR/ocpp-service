//! Change Configuration command

use log::info;
use ocpp_rs::v16::call::{Action, ChangeConfiguration};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::ParsedGenericStatus;

use super::{CommandError, SharedCommandSender};

/// Change a configuration key on a charge point
///
/// Returns the status: Accepted, Rejected, RebootRequired, NotSupported
pub async fn change_configuration(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    key: String,
    value: String,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(
        "[{}] ChangeConfiguration - Key: {}, Value: {}",
        charge_point_id, key, value
    );

    let action = Action::ChangeConfiguration(ChangeConfiguration { key, value });

    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(status_response) => {
            Ok(status_response.get_status().clone())
        }
        ResultPayload::PossibleEmptyResponse(empty_response) => {
            // Some stations may return empty response for accepted
            match empty_response {
                ocpp_rs::v16::call_result::EmptyResponses::EmptyResponse(_) => {
                    Ok(ParsedGenericStatus::Accepted)
                }
                _ => Err(CommandError::InvalidResponse(
                    "Unexpected empty response type".to_string(),
                )),
            }
        }
        _ => Err(CommandError::InvalidResponse(
            "Unexpected response type".to_string(),
        )),
    }
}
