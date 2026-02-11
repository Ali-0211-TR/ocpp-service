//! Remote Start Transaction command

use ocpp_rs::v16::call::{Action, RemoteStartTransaction};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::ParsedGenericStatus;
use tracing::info;

use super::{CommandError, SharedCommandSender};

pub async fn remote_start_transaction(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    id_tag: &str,
    connector_id: Option<u32>,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(charge_point_id, id_tag, ?connector_id, "RemoteStartTransaction");

    let action = Action::RemoteStartTransaction(RemoteStartTransaction {
        id_tag: id_tag.to_string(),
        connector_id,
        charging_profile: None,
    });

    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(sr) => Ok(sr.get_status().clone()),
        _ => Err(CommandError::InvalidResponse("Unexpected response type".to_string())),
    }
}
