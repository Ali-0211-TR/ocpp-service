//! StopTransaction handler

use rust_ocpp::v1_6::messages::stop_transaction::{
    StopTransactionRequest, StopTransactionResponse,
};
use rust_ocpp::v1_6::types::{AuthorizationStatus, IdTagInfo};
use serde_json::Value;
use tracing::{error, info, warn};

use crate::application::events::{Event, TransactionBilledEvent, TransactionStoppedEvent};
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

    // ── IdTag authorization check ──────────────────────────────
    // Verify that the id_tag stopping the transaction matches the one that started it.
    // Per OCPP 1.6 spec §4.8: StopTransaction is always processed, but id_tag_info
    // reflects authorization status (Invalid if mismatch).
    let id_tag_authorized = if let Some(ref stop_id_tag) = req.id_tag {
        match handler.service.get_transaction(transaction_id).await {
            Ok(Some(tx)) => {
                let start_tag = &tx.id_tag;
                if stop_id_tag == start_tag {
                    true
                } else {
                    // Check parent id_tag relationship
                    let parent_match = handler
                        .service
                        .get_id_tag_parent(stop_id_tag)
                        .await
                        .ok()
                        .flatten()
                        .map(|parent| &parent == start_tag)
                        .unwrap_or(false);

                    if parent_match {
                        info!(
                            charge_point_id = handler.charge_point_id.as_str(),
                            transaction_id,
                            stop_id_tag,
                            start_id_tag = start_tag.as_str(),
                            "StopTransaction authorized via parent id_tag"
                        );
                        true
                    } else {
                        warn!(
                            charge_point_id = handler.charge_point_id.as_str(),
                            transaction_id,
                            stop_id_tag,
                            start_id_tag = start_tag.as_str(),
                            "StopTransaction id_tag mismatch — possible unauthorized stop"
                        );
                        false
                    }
                }
            }
            _ => {
                warn!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    transaction_id,
                    "Cannot verify id_tag — transaction not found in DB"
                );
                false
            }
        }
    } else {
        // No id_tag in StopTransaction — allowed per spec (e.g. local stop button)
        true
    };

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

                handler
                    .event_bus
                    .publish(Event::TransactionBilled(TransactionBilledEvent {
                        charge_point_id: handler.charge_point_id.clone(),
                        transaction_id,
                        energy_kwh,
                        duration_minutes: billing.duration_seconds as f64 / 60.0,
                        energy_cost: billing.energy_cost as f64 / 100.0,
                        time_cost: billing.time_cost as f64 / 100.0,
                        session_fee: billing.session_fee as f64 / 100.0,
                        total_cost,
                        currency: currency.clone(),
                        tariff_name: None,
                        timestamp: req.timestamp,
                    }));

                handler
                    .event_bus
                    .publish(Event::TransactionStopped(TransactionStoppedEvent {
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

                handler
                    .event_bus
                    .publish(Event::TransactionStopped(TransactionStoppedEvent {
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

    let auth_status = if id_tag_authorized {
        AuthorizationStatus::Accepted
    } else {
        AuthorizationStatus::Invalid
    };

    let response = StopTransactionResponse {
        id_tag_info: Some(IdTagInfo {
            status: auth_status,
            expiry_date: None,
            parent_id_tag: None,
        }),
    };

    serde_json::to_value(&response).unwrap_or_default()
}
