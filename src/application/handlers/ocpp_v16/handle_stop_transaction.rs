//! StopTransaction handler

use rust_ocpp::v1_6::messages::stop_transaction::{
    StopTransactionRequest, StopTransactionResponse,
};
use rust_ocpp::v1_6::types::{AuthorizationStatus, IdTagInfo};
use serde_json::Value;
use tracing::{error, info};

use crate::application::events::{Event, TransactionStoppedEvent};
use crate::application::OcppHandlerV16;

pub async fn handle_stop_transaction(handler: &OcppHandlerV16, payload: &Value) -> Value {
    let req: StopTransactionRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(charge_point_id = handler.charge_point_id.as_str(), error = %e, "Failed to parse StopTransaction");
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        transaction_id = req.transaction_id,
        meter_stop = req.meter_stop,
        "StopTransaction"
    );

    let transaction_id = req.transaction_id;

    let stop_result = handler
        .service
        .stop_transaction(
            transaction_id,
            req.meter_stop,
            req.reason.as_ref().map(|r| format!("{:?}", r)),
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
                    id_tag: req.id_tag.clone(),
                    meter_stop: req.meter_stop,
                    energy_consumed_kwh: energy_kwh,
                    total_cost,
                    currency,
                    reason: req.reason.as_ref().map(|r| format!("{:?}", r)),
                    timestamp: req.timestamp,
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
                    id_tag: req.id_tag.clone(),
                    meter_stop: req.meter_stop,
                    energy_consumed_kwh: 0.0,
                    total_cost: 0.0,
                    currency: "UZS".to_string(),
                    reason: req.reason.as_ref().map(|r| format!("{:?}", r)),
                    timestamp: req.timestamp,
                }));
            }
        }
    }

    let response = StopTransactionResponse {
        id_tag_info: Some(IdTagInfo {
            status: AuthorizationStatus::Accepted,
            expiry_date: None,
            parent_id_tag: None,
        }),
    };

    serde_json::to_value(&response).unwrap_or_default()
}
