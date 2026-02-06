//! Authorize handler

use chrono::Utc;
use log::info;
use ocpp_rs::v16::call::Authorize;
use ocpp_rs::v16::call_result::{EmptyResponses, GenericIdTagInfo, ResultPayload};
use ocpp_rs::v16::data_types::IdTagInfo;
use ocpp_rs::v16::enums::ParsedGenericStatus;

use crate::application::OcppHandler;
use crate::notifications::{AuthorizationEvent, Event};

pub async fn handle_authorize(handler: &OcppHandler, payload: Authorize) -> ResultPayload {
    info!("[{}] Authorize - IdTag: {}", handler.charge_point_id, payload.id_tag);

    // Get the proper authorization status from the database
    let auth_status = handler.service.get_auth_status(&payload.id_tag).await.ok().flatten();

    let status = match auth_status.as_deref() {
        Some("Accepted") => ParsedGenericStatus::Accepted,
        Some("Blocked") => ParsedGenericStatus::Blocked,
        Some("Expired") => ParsedGenericStatus::Expired,
        Some("ConcurrentTx") => ParsedGenericStatus::ConcurrentTx,
        Some("Invalid") | Some(_) | None => ParsedGenericStatus::Invalid,
    };

    // Publish authorization event
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