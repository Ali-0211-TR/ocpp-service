//! Unlock Connector command

use ocpp_rs::v16::call::{Action, UnlockConnector};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::ParsedGenericStatus;
use tracing::info;

use super::{CommandError, SharedCommandSender};

pub async fn unlock_connector(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    connector_id: u32,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(charge_point_id, connector_id, "UnlockConnector");

    let action = Action::UnlockConnector(UnlockConnector { connector_id });
    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(sr) => Ok(sr.get_status().clone()),
        _ => Err(CommandError::InvalidResponse("Unexpected response type".to_string())),
    }
}
