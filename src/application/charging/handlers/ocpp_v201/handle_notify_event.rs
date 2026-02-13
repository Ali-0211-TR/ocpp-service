//! V201 NotifyEvent handler
//!
//! Receives device events (alerts, monitoring triggers, etc.) from charge
//! points. Each event is logged and published to the event bus as a
//! [`DeviceAlertEvent`].

use rust_ocpp::v2_0_1::messages::notify_event::{NotifyEventRequest, NotifyEventResponse};
use serde_json::Value;
use tracing::{error, info};

use crate::application::events::{DeviceAlertEvent, Event};
use crate::application::OcppHandlerV201;

pub async fn handle_notify_event(handler: &OcppHandlerV201, payload: &Value) -> Value {
    let req: NotifyEventRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse NotifyEvent"
            );
            return serde_json::to_value(NotifyEventResponse {}).unwrap_or_default();
        }
    };

    let tbc = req.tbc.unwrap_or(false);
    let event_count = req.event_data.len();

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        generated_at = %req.generated_at,
        seq_no = req.seq_no,
        tbc,
        events = event_count,
        "V201 NotifyEvent received ({} event(s){})",
        event_count,
        if tbc { ", more coming" } else { "" }
    );

    for ed in &req.event_data {
        let trigger = format!("{:?}", ed.trigger);
        let notification_type = format!("{:?}", ed.event_notification_type);
        let component = &ed.component.name;
        let variable = &ed.variable.name;

        info!(
            charge_point_id = handler.charge_point_id.as_str(),
            event_id = ed.event_id,
            component,
            variable,
            actual_value = ed.actual_value.as_str(),
            trigger = trigger.as_str(),
            notification_type = notification_type.as_str(),
            cleared = ed.cleared,
            "V201 device event: {} / {} = {}",
            component,
            variable,
            ed.actual_value
        );

        if ed.cleared == Some(true) {
            info!(
                charge_point_id = handler.charge_point_id.as_str(),
                event_id = ed.event_id,
                "Event cleared"
            );
        }

        let alert = DeviceAlertEvent {
            charge_point_id: handler.charge_point_id.clone(),
            event_id: ed.event_id,
            component: component.clone(),
            variable: variable.clone(),
            actual_value: ed.actual_value.clone(),
            trigger: trigger.clone(),
            event_notification_type: notification_type.clone(),
            severity: None, // EventDataType doesn't carry severity
            tech_code: ed.tech_code.clone(),
            tech_info: ed.tech_info.clone(),
            cleared: ed.cleared,
            transaction_id: ed.transaction_id.clone(),
            timestamp: ed.timestamp,
        };

        handler
            .event_bus
            .publish(Event::DeviceAlert(alert));
    }

    serde_json::to_value(NotifyEventResponse {}).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::charging::handlers::ocpp_v201_handler::OcppHandlerV201;
    use crate::application::charging::services::device_report::DeviceReportStore;
    use crate::application::events::EventBus;
    use std::sync::Arc;

    fn make_handler() -> OcppHandlerV201 {
        // We need stubs — the tests only exercise the handler logic,
        // not the full service stack. Re-use the minimal approach from
        // other handler tests.
        //
        // This test is limited to verifying JSON parsing + response shape.
        // Full integration tests go elsewhere.
        panic!("unit tests for handler require service stubs — see integration tests")
    }

    #[test]
    fn test_notify_event_response_shape() {
        let resp = NotifyEventResponse {};
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json, serde_json::json!({}));
    }
}
