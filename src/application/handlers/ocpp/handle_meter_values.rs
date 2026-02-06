//! MeterValues handler

use log::info;
use ocpp_rs::v16::call::MeterValues;
use ocpp_rs::v16::call_result::{EmptyResponse, EmptyResponses, ResultPayload};

use crate::application::OcppHandler;
use crate::notifications::{Event, MeterValuesEvent};

pub async fn handle_meter_values(handler: &OcppHandler, payload: MeterValues) -> ResultPayload {
    info!(
        "[{}] MeterValues - Connector: {}, TransactionId: {:?}, Values: {} samples",
        handler.charge_point_id,
        payload.connector_id,
        payload.transaction_id,
        payload.meter_value.len()
    );

    // Publish meter values event for each value
    for meter_value in &payload.meter_value {
        if let Some(sampled_value) = meter_value.sampled_value.first() {
            let value: f64 = sampled_value.value.parse().unwrap_or(0.0);
            let unit = sampled_value
                .unit
                .as_ref()
                .map(|u| format!("{:?}", u))
                .unwrap_or_else(|| "Wh".to_string());

            handler.event_bus.publish(Event::MeterValuesReceived(MeterValuesEvent {
                charge_point_id: handler.charge_point_id.clone(),
                connector_id: payload.connector_id,
                transaction_id: payload.transaction_id.map(|id| id as i32),
                meter_value: value,
                unit,
                timestamp: meter_value.timestamp.inner(),
            }));
        }
    }

    // TODO: Store meter values for analytics
    // This can be used for billing, energy monitoring, etc.

    ResultPayload::PossibleEmptyResponse(EmptyResponses::EmptyResponse(EmptyResponse {}))
}