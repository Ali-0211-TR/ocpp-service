//! Unlock Connector command

use log::info;
use ocpp_rs::v16::call::{Action, UnlockConnector};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::ParsedGenericStatus;

use super::{CommandError, SharedCommandSender};

/// Unlock a connector on a charge point
pub async fn unlock_connector(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    connector_id: u32,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(
        "[{}] UnlockConnector - ConnectorId: {}",
        charge_point_id, connector_id
    );

    let action = Action::UnlockConnector(UnlockConnector { connector_id });

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
