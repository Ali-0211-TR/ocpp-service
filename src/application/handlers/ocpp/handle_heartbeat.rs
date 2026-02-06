//! Heartbeat handler

use chrono::Utc;
use log::info;
use ocpp_rs::v16::call::Heartbeat;
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::data_types::DateTimeWrapper;

use crate::application::OcppHandler;
use crate::notifications::{Event, HeartbeatEvent};

pub async fn handle_heartbeat(handler: &OcppHandler, _payload: Heartbeat) -> ResultPayload {
    info!("[{}] Heartbeat", handler.charge_point_id);

    // Update heartbeat timestamp
    let _ = handler.service.heartbeat(&handler.charge_point_id).await;

    // Publish heartbeat event
    handler.event_bus.publish(Event::HeartbeatReceived(HeartbeatEvent {
        charge_point_id: handler.charge_point_id.clone(),
        timestamp: Utc::now(),
    }));

    ResultPayload::Heartbeat(ocpp_rs::v16::call_result::Heartbeat {
        current_time: DateTimeWrapper::new(Utc::now()),
    })
}