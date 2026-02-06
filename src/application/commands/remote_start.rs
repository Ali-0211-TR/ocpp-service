//! Remote Start Transaction command

use log::info;
use ocpp_rs::v16::call::{Action, RemoteStartTransaction};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::ParsedGenericStatus;

use super::{CommandError, SharedCommandSender};

/// Start a charging transaction remotely
pub async fn remote_start_transaction(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    id_tag: &str,
    connector_id: Option<u32>,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(
        "[{}] RemoteStartTransaction - IdTag: {}, Connector: {:?}",
        charge_point_id, id_tag, connector_id
    );

    let action = Action::RemoteStartTransaction(RemoteStartTransaction {
        id_tag: id_tag.to_string(),
        connector_id,
        charging_profile: None,
    });

    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(status_response) => {
            // All responses map to StatusResponse variant with GenericStatusResponse
            Ok(status_response.get_status().clone())
        }
        _ => Err(CommandError::InvalidResponse(
            "Unexpected response type".to_string(),
        )),
    }
}
