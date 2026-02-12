//! V201 Authorize handler

use chrono::Utc;
use rust_ocpp::v2_0_1::datatypes::id_token_info_type::IdTokenInfoType;
use rust_ocpp::v2_0_1::enumerations::authorization_status_enum_type::AuthorizationStatusEnumType;
use rust_ocpp::v2_0_1::messages::authorize::{AuthorizeRequest, AuthorizeResponse};
use serde_json::Value;
use tracing::{error, info};

use crate::application::events::{AuthorizationEvent, Event};
use crate::application::OcppHandlerV201;

pub async fn handle_authorize(handler: &OcppHandlerV201, payload: &Value) -> Value {
    let req: AuthorizeRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse Authorize"
            );
            return serde_json::json!({});
        }
    };

    let id_tag = &req.id_token.id_token;
    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        id_token = id_tag.as_str(),
        token_type = ?req.id_token.kind,
        "V201 Authorize"
    );

    let auth_status = handler.service.get_auth_status(id_tag).await.ok().flatten();

    let status = match auth_status.as_deref() {
        Some("Accepted") => AuthorizationStatusEnumType::Accepted,
        Some("Blocked") => AuthorizationStatusEnumType::Blocked,
        Some("Expired") => AuthorizationStatusEnumType::Expired,
        Some("ConcurrentTx") => AuthorizationStatusEnumType::ConcurrentTx,
        Some("Invalid") | Some(_) | None => AuthorizationStatusEnumType::Invalid,
    };

    handler
        .event_bus
        .publish(Event::AuthorizationResult(AuthorizationEvent {
            charge_point_id: handler.charge_point_id.clone(),
            id_tag: id_tag.clone(),
            status: format!("{:?}", status),
            timestamp: Utc::now(),
        }));

    let response = AuthorizeResponse {
        certificate_status: None,
        id_token_info: IdTokenInfoType {
            status,
            cache_expiry_date_time: None,
            charging_priority: None,
            language1: None,
            evse_id: None,
            language2: None,
            group_id_token: None,
            personal_message: None,
        },
    };

    serde_json::to_value(&response).unwrap_or_default()
}
