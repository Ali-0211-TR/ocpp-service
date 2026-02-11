//! Remote Stop Transaction command

use ocpp_rs::v16::call::{Action, RemoteStopTransaction};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::ParsedGenericStatus;
use tracing::info;

use super::{CommandError, SharedCommandSender};

pub async fn remote_stop_transaction(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    transaction_id: i32,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(charge_point_id, transaction_id, "RemoteStopTransaction");

    let action = Action::RemoteStopTransaction(RemoteStopTransaction { transaction_id });
    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(sr) => Ok(sr.get_status().clone()),
        _ => Err(CommandError::InvalidResponse("Unexpected response type".to_string())),
    }
}
