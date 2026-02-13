//! v1.6 ReserveNow command

use chrono::{DateTime, Utc};
use rust_ocpp::v1_6::messages::reserve_now::{ReserveNowRequest, ReserveNowResponse};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

pub async fn reserve_now(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    reservation_id: i32,
    connector_id: i32,
    id_tag: &str,
    parent_id_tag: Option<&str>,
    expiry_date: DateTime<Utc>,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        reservation_id,
        connector_id,
        id_tag,
        "v1.6 ReserveNow"
    );

    let request = ReserveNowRequest {
        connector_id: connector_id as u32,
        expiry_date,
        id_tag: id_tag.to_string(),
        parent_id_tag: parent_id_tag.map(|s| s.to_string()),
        reservation_id,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "ReserveNow", payload)
        .await?;

    let response: ReserveNowResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
