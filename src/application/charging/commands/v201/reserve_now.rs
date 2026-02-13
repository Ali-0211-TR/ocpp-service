//! v2.0.1 ReserveNow command

use chrono::{DateTime, Utc};
use rust_ocpp::v2_0_1::datatypes::id_token_type::IdTokenType;
use rust_ocpp::v2_0_1::enumerations::id_token_enum_type::IdTokenEnumType;
use rust_ocpp::v2_0_1::messages::reserve_now::{ReserveNowRequest, ReserveNowResponse};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

pub async fn reserve_now(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    reservation_id: i32,
    evse_id: Option<i32>,
    id_tag: &str,
    expiry_date: DateTime<Utc>,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        reservation_id,
        ?evse_id,
        id_tag,
        "v2.0.1 ReserveNow"
    );

    let request = ReserveNowRequest {
        id: reservation_id,
        expiry_date_time: expiry_date,
        connector_type: None,
        evse_id,
        id_token: IdTokenType {
            id_token: id_tag.to_string(),
            kind: IdTokenEnumType::Central,
            additional_info: None,
        },
        group_id_token: None,
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
