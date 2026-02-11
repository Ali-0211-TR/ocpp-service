//! StopTransaction handler

use ocpp_rs::v16::call::StopTransaction;
use ocpp_rs::v16::call_result::{EmptyResponses, GenericIdTagInfo, ResultPayload};
use ocpp_rs::v16::data_types::IdTagInfo;
use ocpp_rs::v16::enums::ParsedGenericStatus;
use tracing::{error, info};

use crate::application::events::{Event, TransactionStoppedEvent};
use crate::application::OcppHandler;

pub async fn handle_stop_transaction(
    handler: &OcppHandler,
    payload: StopTransaction,
) -> ResultPayload {
    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        transaction_id = payload.transaction_id,
        meter_stop = payload.meter_stop,
        "StopTransaction"
    );

    let transaction_id = payload.transaction_id as i32;

    let stop_result = handler
        .service
        .stop_transaction(
            transaction_id,
            payload.meter_stop as i32,
            payload.reason.as_ref().map(|r| format!("{:?}", r)),
        )
        .await;

    if let Err(e) = &stop_result {
        error!(
            charge_point_id = handler.charge_point_id.as_str(),
            transaction_id,
            error = %e,
            "Failed to stop transaction"
        );
    }

    if stop_result.is_ok() {
        match handler
            .billing_service
            .calculate_transaction_billing(transaction_id, None)
            .await
        {
            Ok(billing) => {
                let energy_kwh = billing.energy_wh as f64 / 1000.0;
                let total_cost = billing.total_cost as f64 / 100.0;
                let currency = billing.currency.clone();

                info!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    transaction_id,
                    total_cost,
                    currency = currency.as_str(),
                    energy_kwh,
                    "Transaction billing calculated"
                );

                handler.event_bus.publish(Event::TransactionStopped(TransactionStoppedEvent {
                    charge_point_id: handler.charge_point_id.clone(),
                    transaction_id,
                    id_tag: payload.id_tag.clone(),
                    meter_stop: payload.meter_stop as i32,
                    energy_consumed_kwh: energy_kwh,
                    total_cost,
                    currency,
                    reason: payload.reason.as_ref().map(|r| format!("{:?}", r)),
                    timestamp: payload.timestamp.inner(),
                }));
            }
            Err(e) => {
                error!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    transaction_id,
                    error = %e,
                    "Failed to calculate billing"
                );

                handler.event_bus.publish(Event::TransactionStopped(TransactionStoppedEvent {
                    charge_point_id: handler.charge_point_id.clone(),
                    transaction_id,
                    id_tag: payload.id_tag.clone(),
                    meter_stop: payload.meter_stop as i32,
                    energy_consumed_kwh: 0.0,
                    total_cost: 0.0,
                    currency: "UZS".to_string(),
                    reason: payload.reason.as_ref().map(|r| format!("{:?}", r)),
                    timestamp: payload.timestamp.inner(),
                }));
            }
        }
    }

    ResultPayload::PossibleEmptyResponse(EmptyResponses::GenericIdTagInfoResponse(
        GenericIdTagInfo {
            id_tag_info: Some(IdTagInfo {
                status: ParsedGenericStatus::Accepted,
                expiry_date: None,
                parent_id_tag: None,
            }),
        },
    ))
}
