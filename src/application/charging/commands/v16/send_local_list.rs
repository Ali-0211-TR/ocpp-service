//! v1.6 SendLocalList command

use rust_ocpp::v1_6::messages::send_local_list::{SendLocalListRequest, SendLocalListResponse};
use rust_ocpp::v1_6::types::{AuthorizationData, IdTagInfo, UpdateType};
use chrono::DateTime;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Authorization entry for the local list (version-agnostic input).
pub use super::super::LocalAuthEntry;

/// Send a local authorization list to the charge point.
///
/// `update_type`: `"Full"` or `"Differential"`.
pub async fn send_local_list(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    list_version: i32,
    update_type: &str,
    entries: Option<Vec<LocalAuthEntry>>,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        list_version, update_type, "v1.6 SendLocalList"
    );

    let ocpp_update_type = match update_type.to_lowercase().as_str() {
        "differential" => UpdateType::Differential,
        _ => UpdateType::Full,
    };

    let local_authorization_list = entries.map(|list| {
        list.into_iter()
            .map(|e| AuthorizationData {
                id_tag: e.id_tag,
                id_tag_info: e.status.map(|s| {
                    let auth_status = match s.to_lowercase().as_str() {
                        "accepted" => {
                            rust_ocpp::v1_6::types::AuthorizationStatus::Accepted
                        }
                        "blocked" => {
                            rust_ocpp::v1_6::types::AuthorizationStatus::Blocked
                        }
                        "expired" => {
                            rust_ocpp::v1_6::types::AuthorizationStatus::Expired
                        }
                        "invalid" => {
                            rust_ocpp::v1_6::types::AuthorizationStatus::Invalid
                        }
                        "concurrenttx" | "concurrent_tx" => {
                            rust_ocpp::v1_6::types::AuthorizationStatus::ConcurrentTx
                        }
                        _ => rust_ocpp::v1_6::types::AuthorizationStatus::Accepted,
                    };
                    IdTagInfo {
                        status: auth_status,
                        expiry_date: e.expiry_date.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&chrono::Utc))),
                        parent_id_tag: e.parent_id_tag,
                    }
                }),
            })
            .collect()
    });

    let request = SendLocalListRequest {
        list_version,
        local_authorization_list,
        update_type: ocpp_update_type,
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
