//! Reset command

use log::info;
use ocpp_rs::v16::call::{Action, Reset};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::{ParsedGenericStatus, ResetType};

use super::{CommandError, SharedCommandSender};

/// Reset type for the charge point
#[derive(Debug, Clone, Copy)]
pub enum ResetKind {
    /// Soft reset - restart without power cycle
    Soft,
    /// Hard reset - full power cycle
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

/// Reset a charge point
pub async fn reset(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    reset_type: ResetKind,
) -> Result<ParsedGenericStatus, CommandError> {
    info!("[{}] Reset - Type: {:?}", charge_point_id, reset_type);

    let action = Action::Reset(Reset {
        reset_type: reset_type.into(),
    });

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
