//! v1.6 Reset command

use rust_ocpp::v1_6::messages::reset::{ResetRequest, ResetResponse};
use rust_ocpp::v1_6::types::ResetRequestStatus;
use tracing::info;

use crate::application::charging::commands::{CommandError, ResetKind, SharedCommandSender};

pub async fn reset(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    reset_type: ResetKind,
) -> Result<String, CommandError> {
    info!(charge_point_id, ?reset_type, "v1.6 Reset");

    let kind = match reset_type {
        ResetKind::Soft => ResetRequestStatus::Soft,
        ResetKind::Hard => ResetRequestStatus::Hard,
    };

    let request = ResetRequest { kind };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "Reset", payload)
        .await?;

    let response: ResetResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
