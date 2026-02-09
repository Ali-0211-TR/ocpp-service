//! Clear Cache command

use log::info;
use ocpp_rs::v16::call::{Action, ClearCache};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::ParsedGenericStatus;

use super::{CommandError, SharedCommandSender};

/// Clear the authorization cache on a charge point
///
/// Instructs the charge point to clear its local authorization cache.
/// Returns Accepted or Rejected.
pub async fn clear_cache(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
) -> Result<ParsedGenericStatus, CommandError> {
    info!("[{}] ClearCache", charge_point_id);

    let action = Action::ClearCache(ClearCache {});

    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(status_response) => {
            Ok(status_response.get_status().clone())
        }
        ResultPayload::PossibleEmptyResponse(empty_response) => match empty_response {
            ocpp_rs::v16::call_result::EmptyResponses::EmptyResponse(_) => {
                Ok(ParsedGenericStatus::Accepted)
            }
            _ => Err(CommandError::InvalidResponse(
                "Unexpected empty response type".to_string(),
            )),
        },
        _ => Err(CommandError::InvalidResponse(
            "Unexpected response type".to_string(),
        )),
    }
}
