//! Get Local List Version command

use log::info;
use ocpp_rs::v16::call::{Action, GetLocalListVersion};
use ocpp_rs::v16::call_result::ResultPayload;

use super::{CommandError, SharedCommandSender};

/// Get the version of the local authorization list on a charge point
///
/// Returns the list version number. -1 means the list is not supported.
/// 0 means the list is empty.
pub async fn get_local_list_version(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
) -> Result<i32, CommandError> {
    info!("[{}] GetLocalListVersion", charge_point_id);

    let action = Action::GetLocalListVersion(GetLocalListVersion {});

    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::GetLocalListVersion(resp) => Ok(resp.list_version),
        _ => Err(CommandError::InvalidResponse(
            "Unexpected response type".to_string(),
        )),
    }
}
