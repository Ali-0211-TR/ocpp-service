//! Authorize handler

use chrono::Utc;
use rust_ocpp::v1_6::messages::authorize::{AuthorizeRequest, AuthorizeResponse};
use rust_ocpp::v1_6::types::{AuthorizationStatus, IdTagInfo};
use serde_json::Value;
use tracing::{error, info};

use crate::application::events::{AuthorizationEvent, Event};
use crate::application::OcppHandlerV16;

pub async fn handle_authorize(handler: &OcppHandlerV16, payload: &Value) -> Value {
    let req: AuthorizeRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(charge_point_id = handler.charge_point_id.as_str(), error = %e, "Failed to parse Authorize");
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        id_tag = req.id_tag.as_str(),
        "Authorize"
    );

    let auth_status = handler
        .service
        .get_auth_status(&req.id_tag)
        .await
        .ok()
        .flatten();

    let status = match auth_status.as_deref() {
        Some("Accepted") => AuthorizationStatus::Accepted,
        Some("Blocked") => AuthorizationStatus::Blocked,
        Some("Expired") => AuthorizationStatus::Expired,
        Some("ConcurrentTx") => AuthorizationStatus::ConcurrentTx,
        Some("Invalid") | Some(_) | None => AuthorizationStatus::Invalid,
    };

    handler
        .event_bus
        .publish(Event::AuthorizationResult(AuthorizationEvent {
            charge_point_id: handler.charge_point_id.clone(),
            id_tag: req.id_tag.clone(),
            status: format!("{:?}", status),
            timestamp: Utc::now(),
        }));

    let response = AuthorizeResponse {
        id_tag_info: IdTagInfo {
            status,
            expiry_date: None,
            parent_id_tag: None,
        },
    };

    serde_json::to_value(&response).unwrap_or_default()
}
