//! v2.0.1 RequestStartTransaction command

use rust_ocpp::v2_0_1::messages::request_start_transaction::{
    RequestStartTransactionRequest, RequestStartTransactionResponse,
};
use rust_ocpp::v2_0_1::datatypes::id_token_type::IdTokenType;
use rust_ocpp::v2_0_1::enumerations::id_token_enum_type::IdTokenEnumType;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

pub async fn remote_start_transaction(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    id_tag: &str,
    evse_id: Option<i32>,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        id_tag,
        ?evse_id,
        "v2.0.1 RequestStartTransaction"
    );

    let request = RequestStartTransactionRequest {
        evse_id,
        remote_start_id: 1, // Auto-generated unique id
        id_token: IdTokenType {
            id_token: id_tag.to_string(),
            kind: IdTokenEnumType::Central,
            additional_info: None,
        },
        charging_profile: None,
        group_id_token: None,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "RequestStartTransaction", payload)
        .await?;

    let response: RequestStartTransactionResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
