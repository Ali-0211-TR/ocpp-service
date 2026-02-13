//! v2.0.1 SendLocalList command

use chrono::DateTime;
use rust_ocpp::v2_0_1::datatypes::authorization_data::AuthorizationData;
use rust_ocpp::v2_0_1::datatypes::id_token_info_type::IdTokenInfoType;
use rust_ocpp::v2_0_1::datatypes::id_token_type::IdTokenType;
use rust_ocpp::v2_0_1::enumerations::authorization_status_enum_type::AuthorizationStatusEnumType;
use rust_ocpp::v2_0_1::enumerations::id_token_enum_type::IdTokenEnumType;
use rust_ocpp::v2_0_1::enumerations::update_enum_type::UpdateEnumType;
use rust_ocpp::v2_0_1::messages::send_local_list::{SendLocalListRequest, SendLocalListResponse};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Authorization entry for the local list (version-agnostic input).
pub use super::super::LocalAuthEntry;

/// Send a local authorization list to the charging station (v2.0.1).
///
/// `update_type`: `"Full"` or `"Differential"`.
pub async fn send_local_list(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    version_number: i32,
    update_type: &str,
    entries: Option<Vec<LocalAuthEntry>>,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        version_number, update_type, "v2.0.1 SendLocalList"
    );

    let ocpp_update_type = match update_type.to_lowercase().as_str() {
        "differential" => UpdateEnumType::Differential,
        _ => UpdateEnumType::Full,
    };

    let local_authorization_list = entries.map(|list| {
        list.into_iter()
            .map(|e| {
                let id_token = IdTokenType {
                    id_token: e.id_tag.clone(),
                    kind: IdTokenEnumType::Central,
                    additional_info: None,
                };

                let id_token_info = e.status.map(|s| {
                    let auth_status = match s.to_lowercase().as_str() {
                        "accepted" => AuthorizationStatusEnumType::Accepted,
                        "blocked" => AuthorizationStatusEnumType::Blocked,
                        "expired" => AuthorizationStatusEnumType::ConcurrentTx,
                        "invalid" => AuthorizationStatusEnumType::Invalid,
                        "noallowedcredit" | "no_allowed_credit" => {
                            AuthorizationStatusEnumType::Invalid
                        }
                        _ => AuthorizationStatusEnumType::Accepted,
                    };

                    IdTokenInfoType {
                        status: auth_status,
                        cache_expiry_date_time: e.expiry_date.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&chrono::Utc))),
                        charging_priority: None,
                        language1: None,
                        language2: None,
                        evse_id: None,
                        group_id_token: None,
                        personal_message: None,
                    }
                });

                AuthorizationData {
                    id_token,
                    id_token_info,
                }
            })
            .collect()
    });

    let request = SendLocalListRequest {
        version_number,
        update_type: ocpp_update_type,
        local_authorization_list,
    };

    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "SendLocalList", payload)
        .await?;

    let response: SendLocalListResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
