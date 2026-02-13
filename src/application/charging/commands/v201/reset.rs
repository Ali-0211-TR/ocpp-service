//! v2.0.1 Reset command

use rust_ocpp::v2_0_1::messages::reset::{ResetRequest, ResetResponse};
use rust_ocpp::v2_0_1::enumerations::reset_enum_type::ResetEnumType;
use tracing::info;

use crate::application::charging::commands::{CommandError, ResetKind, SharedCommandSender};

pub async fn reset(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    reset_type: ResetKind,
    evse_id: Option<i32>,
) -> Result<String, CommandError> {
    info!(charge_point_id, ?reset_type, ?evse_id, "v2.0.1 Reset");

    let request_type = match reset_type {
        // v1.6 Hard → v2.0.1 Immediate
        ResetKind::Hard => ResetEnumType::Immediate,
        // v1.6 Soft → v2.0.1 OnIdle
        ResetKind::Soft => ResetEnumType::OnIdle,
    };

    let request = ResetRequest {
        request_type,
        evse_id,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "Reset", payload)
        .await?;

    let response: ResetResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
