//! Reset command

use ocpp_rs::v16::call::{Action, Reset};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::{ParsedGenericStatus, ResetType};
use tracing::info;

use super::{CommandError, SharedCommandSender};

#[derive(Debug, Clone, Copy)]
pub enum ResetKind {
    Soft,
    Hard,
}

impl From<ResetKind> for ResetType {
    fn from(kind: ResetKind) -> Self {
        match kind {
            ResetKind::Soft => ResetType::Soft,
            ResetKind::Hard => ResetType::Hard,
        }
    }
}

pub async fn reset(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    reset_type: ResetKind,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(charge_point_id, ?reset_type, "Reset");

    let action = Action::Reset(Reset {
        reset_type: reset_type.into(),
    });
    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(sr) => Ok(sr.get_status().clone()),
        _ => Err(CommandError::InvalidResponse("Unexpected response type".to_string())),
    }
}
