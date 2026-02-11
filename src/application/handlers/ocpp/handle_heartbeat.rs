//! Heartbeat handler

use chrono::Utc;
use ocpp_rs::v16::call::Heartbeat;
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::data_types::DateTimeWrapper;
use tracing::info;

use crate::application::events::{Event, HeartbeatEvent};
use crate::application::OcppHandler;

pub async fn handle_heartbeat(handler: &OcppHandler, _payload: Heartbeat) -> ResultPayload {
    info!(charge_point_id = handler.charge_point_id.as_str(), "Heartbeat");

    let _ = handler.service.heartbeat(&handler.charge_point_id).await;

    handler.event_bus.publish(Event::HeartbeatReceived(HeartbeatEvent {
        charge_point_id: handler.charge_point_id.clone(),
        timestamp: Utc::now(),
    }));

    ResultPayload::Heartbeat(ocpp_rs::v16::call_result::Heartbeat {
        current_time: DateTimeWrapper::new(Utc::now()),
    })
}
