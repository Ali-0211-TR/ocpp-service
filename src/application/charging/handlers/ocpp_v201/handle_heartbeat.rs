//! V201 Heartbeat handler

use chrono::Utc;
use rust_ocpp::v2_0_1::messages::heartbeat::HeartbeatResponse;
use serde_json::Value;
use tracing::info;

use crate::application::events::{Event, HeartbeatEvent};
use crate::application::OcppHandlerV201;

pub async fn handle_heartbeat(handler: &OcppHandlerV201, _payload: &Value) -> Value {
    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        "V201 Heartbeat"
    );

    let _ = handler.service.heartbeat(&handler.charge_point_id).await;

    handler
        .event_bus
        .publish(Event::HeartbeatReceived(HeartbeatEvent {
            charge_point_id: handler.charge_point_id.clone(),
            timestamp: Utc::now(),
        }));

    let response = HeartbeatResponse {
        current_time: Utc::now(),
    };

    serde_json::to_value(&response).unwrap_or_default()
}
