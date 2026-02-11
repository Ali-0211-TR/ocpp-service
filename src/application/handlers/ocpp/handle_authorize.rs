//! Authorize handler

use chrono::Utc;
use ocpp_rs::v16::call::Authorize;
use ocpp_rs::v16::call_result::{EmptyResponses, GenericIdTagInfo, ResultPayload};
use ocpp_rs::v16::data_types::IdTagInfo;
use ocpp_rs::v16::enums::ParsedGenericStatus;
use tracing::info;

use crate::application::events::{AuthorizationEvent, Event};
use crate::application::OcppHandler;

pub async fn handle_authorize(handler: &OcppHandler, payload: Authorize) -> ResultPayload {
    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        id_tag = payload.id_tag.as_str(),
        "Authorize"
    );

    let auth_status = handler
        .service
        .get_auth_status(&payload.id_tag)
        .await
        .ok()
        .flatten();

    let status = match auth_status.as_deref() {
        Some("Accepted") => ParsedGenericStatus::Accepted,
        Some("Blocked") => ParsedGenericStatus::Blocked,
        Some("Expired") => ParsedGenericStatus::Expired,
        Some("ConcurrentTx") => ParsedGenericStatus::ConcurrentTx,
        Some("Invalid") | Some(_) | None => ParsedGenericStatus::Invalid,
    };

    handler.event_bus.publish(Event::AuthorizationResult(AuthorizationEvent {
        charge_point_id: handler.charge_point_id.clone(),
        id_tag: payload.id_tag.clone(),
        status: format!("{:?}", status),
        timestamp: Utc::now(),
    }));

    ResultPayload::PossibleEmptyResponse(EmptyResponses::GenericIdTagInfoResponse(
        GenericIdTagInfo {
            id_tag_info: Some(IdTagInfo {
                status,
                expiry_date: None,
                parent_id_tag: None,
            }),
        },
    ))
}
