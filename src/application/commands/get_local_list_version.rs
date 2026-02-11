//! Get Local List Version command

use ocpp_rs::v16::call::{Action, GetLocalListVersion};
use ocpp_rs::v16::call_result::ResultPayload;
use tracing::info;

use super::{CommandError, SharedCommandSender};

pub async fn get_local_list_version(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
) -> Result<i32, CommandError> {
    info!(charge_point_id, "GetLocalListVersion");

    let action = Action::GetLocalListVersion(GetLocalListVersion {});
    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::GetLocalListVersion(resp) => Ok(resp.list_version),
        _ => Err(CommandError::InvalidResponse("Unexpected response type".to_string())),
    }
}
