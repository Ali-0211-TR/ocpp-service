//! V201 TransactionEvent handler
//!
//! In OCPP 2.0.1, `TransactionEvent` replaces the V1.6 `StartTransaction`
//! and `StopTransaction` messages. The `event_type` field determines whether
//! the event is Started, Updated, or Ended.
//!
//! - **Started**: A new transaction begins (equivalent to V1.6 StartTransaction)
//! - **Updated**: Meter values or charging state changed (mid-transaction update)
//! - **Ended**: Transaction has stopped (equivalent to V1.6 StopTransaction)

use rust_ocpp::v2_0_1::datatypes::id_token_info_type::IdTokenInfoType;
use rust_ocpp::v2_0_1::enumerations::authorization_status_enum_type::AuthorizationStatusEnumType;
use rust_ocpp::v2_0_1::enumerations::measurand_enum_type::MeasurandEnumType;
use rust_ocpp::v2_0_1::enumerations::transaction_event_enum_type::TransactionEventEnumType;
use rust_ocpp::v2_0_1::messages::transaction_event::{
    TransactionEventRequest, TransactionEventResponse,
};
use serde_json::Value;
use tracing::{error, info, warn};

use crate::application::events::{
    Event, MeterValuesEvent, TransactionStartedEvent, TransactionStoppedEvent,
};
use crate::application::handlers::OcppHandlerV201;

pub async fn handle_transaction_event(handler: &OcppHandlerV201, payload: &Value) -> Value {
    let req: TransactionEventRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse TransactionEvent"
            );
            return serde_json::json!({});
        }
    };

    let tx_id_str = &req.transaction_info.transaction_id;
    let evse_id = req.evse.as_ref().map(|e| e.id).unwrap_or(1);
    let id_tag = req
        .id_token
        .as_ref()
        .map(|t| t.id_token.clone())
        .unwrap_or_default();

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        event_type = ?req.event_type,
        transaction_id = tx_id_str.as_str(),
        trigger_reason = ?req.trigger_reason,
        evse_id,
        "V201 TransactionEvent"
    );

    // Extract meter values if present
    let (energy_wh, power_w, soc) = extract_meter_values(&req);

    match req.event_type {
        TransactionEventEnumType::Started => {
            handle_started(handler, &req, evse_id as u32, &id_tag, energy_wh).await
        }
        TransactionEventEnumType::Updated => {
            handle_updated(handler, &req, evse_id as u32, energy_wh, power_w, soc).await
        }
        TransactionEventEnumType::Ended => {
            handle_ended(handler, &req, evse_id as u32, &id_tag, energy_wh).await
        }
    }
}

/// Handle TransactionEvent with event_type = Started
async fn handle_started(
    handler: &OcppHandlerV201,
    req: &TransactionEventRequest,
    evse_id: u32,
    id_tag: &str,
    energy_wh: Option<f64>,
) -> Value {
    let meter_start = energy_wh.unwrap_or(0.0) as i32;

    let is_valid = if !id_tag.is_empty() {
        handler.service.authorize(id_tag).await.unwrap_or(false)
    } else {
        // No id_token means possibly remote-started or no auth required
        true
    };

    if !is_valid {
        return build_response(Some(AuthorizationStatusEnumType::Invalid));
    }

    match handler
        .service
        .start_transaction(
            &handler.charge_point_id,
            evse_id,
            if id_tag.is_empty() { "unknown" } else { id_tag },
            meter_start,
        )
        .await
    {
        Ok(transaction) => {
            handler
                .event_bus
                .publish(Event::TransactionStarted(TransactionStartedEvent {
                    charge_point_id: handler.charge_point_id.clone(),
                    connector_id: evse_id,
                    transaction_id: transaction.id,
                    id_tag: id_tag.to_string(),
                    meter_start,
                    timestamp: req.timestamp,
                }));

            build_response(Some(AuthorizationStatusEnumType::Accepted))
        }
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to start transaction"
            );
            build_response(Some(AuthorizationStatusEnumType::Invalid))
        }
    }
}

/// Handle TransactionEvent with event_type = Updated
async fn handle_updated(
    handler: &OcppHandlerV201,
    req: &TransactionEventRequest,
    evse_id: u32,
    energy_wh: Option<f64>,
    power_w: Option<f64>,
    soc: Option<f64>,
) -> Value {
    let tx_id_str = &req.transaction_info.transaction_id;

    // Find the active transaction for this EVSE
    match handler
        .service
        .get_active_transaction_for_connector(&handler.charge_point_id, evse_id)
        .await
    {
        Ok(Some(tx)) => {
            let _ = handler
                .service
                .update_transaction_meter_data(
                    tx.id,
                    energy_wh.map(|e| e as i32),
                    power_w,
                    soc.map(|s| s as i32),
                )
                .await;

            // Check if limit is reached, send remote stop if needed
            if let Ok(Some(updated_tx)) = handler
                .service
                .get_active_transaction_for_connector(&handler.charge_point_id, evse_id)
                .await
            {
                if updated_tx.is_limit_reached() {
                    warn!(
                        charge_point_id = handler.charge_point_id.as_str(),
                        transaction_id = tx.id,
                        limit_type = ?updated_tx.limit_type,
                        limit_value = ?updated_tx.limit_value,
                        "V201: Charging limit reached! Sending RequestStopTransaction."
                    );

                    let stop_payload = serde_json::json!({
                        "transactionId": tx_id_str,
                    });
                    if let Err(e) = handler
                        .command_sender
                        .send_command(
                            &handler.charge_point_id,
                            "RequestStopTransaction",
                            stop_payload,
                        )
                        .await
                    {
                        error!(
                            charge_point_id = handler.charge_point_id.as_str(),
                            error = ?e,
                            "V201: Failed to send RequestStopTransaction"
                        );
                    }
                }
            }

            // Compute consumed energy from meter_start
            let energy_consumed_wh = energy_wh.map(|e| e - tx.meter_start as f64);

            handler
                .event_bus
                .publish(Event::MeterValuesReceived(MeterValuesEvent {
                    charge_point_id: handler.charge_point_id.clone(),
                    connector_id: evse_id,
                    transaction_id: Some(tx.id),
                    energy_wh,
                    energy_consumed_wh,
                    power_w,
                    soc,
                    timestamp: req.timestamp,
                }));
        }
        Ok(None) => {
            warn!(
                charge_point_id = handler.charge_point_id.as_str(),
                v201_tx_id = tx_id_str.as_str(),
                evse_id,
                "V201: No active transaction found for Updated event"
            );
        }
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to look up transaction for Updated event"
            );
        }
    }

    build_response(None)
}

/// Handle TransactionEvent with event_type = Ended
async fn handle_ended(
    handler: &OcppHandlerV201,
    req: &TransactionEventRequest,
    evse_id: u32,
    id_tag: &str,
    energy_wh: Option<f64>,
) -> Value {
    let tx_id_str = &req.transaction_info.transaction_id;
    let reason = req
        .transaction_info
        .stopped_reason
        .as_ref()
        .map(|r| format!("{:?}", r));

    // Find the active transaction for this EVSE
    match handler
        .service
        .get_active_transaction_for_connector(&handler.charge_point_id, evse_id)
        .await
    {
        Ok(Some(tx)) => {
            let meter_stop = energy_wh.unwrap_or(tx.meter_start as f64) as i32;

            let stop_result = handler
                .service
                .stop_transaction(tx.id, meter_stop, reason.clone())
                .await;

            if let Err(e) = &stop_result {
                error!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    transaction_id = tx.id,
                    error = %e,
                    "V201: Failed to stop transaction"
                );
            }

            if stop_result.is_ok() {
                match handler
                    .billing_service
                    .calculate_transaction_billing(tx.id, None)
                    .await
                {
                    Ok(billing) => {
                        let energy_kwh = billing.energy_wh as f64 / 1000.0;
                        let total_cost = billing.total_cost as f64 / 100.0;
                        let currency = billing.currency.clone();

                        info!(
                            charge_point_id = handler.charge_point_id.as_str(),
                            transaction_id = tx.id,
                            total_cost,
                            currency = currency.as_str(),
                            energy_kwh,
                            "V201: Transaction billing calculated"
                        );

                        handler.event_bus.publish(Event::TransactionStopped(
                            TransactionStoppedEvent {
                                charge_point_id: handler.charge_point_id.clone(),
                                transaction_id: tx.id,
                                id_tag: if id_tag.is_empty() {
                                    None
                                } else {
                                    Some(id_tag.to_string())
                                },
                                meter_stop,
                                energy_consumed_kwh: energy_kwh,
                                total_cost,
                                currency,
                                reason: reason.clone(),
                                timestamp: req.timestamp,
                            },
                        ));
                    }
                    Err(e) => {
                        error!(
                            charge_point_id = handler.charge_point_id.as_str(),
                            transaction_id = tx.id,
                            error = %e,
                            "V201: Failed to calculate billing"
                        );

                        handler.event_bus.publish(Event::TransactionStopped(
                            TransactionStoppedEvent {
                                charge_point_id: handler.charge_point_id.clone(),
                                transaction_id: tx.id,
                                id_tag: if id_tag.is_empty() {
                                    None
                                } else {
                                    Some(id_tag.to_string())
                                },
                                meter_stop,
                                energy_consumed_kwh: 0.0,
                                total_cost: 0.0,
                                currency: "UZS".to_string(),
                                reason,
                                timestamp: req.timestamp,
                            },
                        ));
                    }
                }
            }
        }
        Ok(None) => {
            warn!(
                charge_point_id = handler.charge_point_id.as_str(),
                v201_tx_id = tx_id_str.as_str(),
                evse_id,
                "V201: No active transaction found for Ended event"
            );
        }
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to look up transaction for Ended event"
            );
        }
    }

    build_response(None)
}

/// Extract energy, power, and SoC from meter values in the request.
fn extract_meter_values(req: &TransactionEventRequest) -> (Option<f64>, Option<f64>, Option<f64>) {
    let mut energy_wh: Option<f64> = None;
    let mut power_w: Option<f64> = None;
    let mut soc: Option<f64> = None;

    if let Some(meter_values) = &req.meter_value {
        for mv in meter_values {
            for sampled in &mv.sampled_value {
                use rust_decimal::prelude::ToPrimitive;
                let value: f64 = match sampled.value.to_f64() {
                    Some(v) => v,
                    None => continue,
                };

                let measurand = sampled
                    .measurand
                    .clone()
                    .unwrap_or(MeasurandEnumType::EnergyActiveImportRegister);

                match measurand {
                    MeasurandEnumType::EnergyActiveImportRegister => {
                        // V201 SampledValue uses UnitOfMeasureType (not an enum).
                        // Default unit for energy is Wh.
                        // Check if unit_of_measure specifies "kWh"
                        let is_kwh = sampled
                            .unit_of_measure
                            .as_ref()
                            .and_then(|u| u.unit.as_ref())
                            .map(|u| u.eq_ignore_ascii_case("kWh"))
                            .unwrap_or(false);
                        energy_wh = Some(if is_kwh { value * 1000.0 } else { value });
                    }
                    MeasurandEnumType::PowerActiveImport => {
                        let is_kw = sampled
                            .unit_of_measure
                            .as_ref()
                            .and_then(|u| u.unit.as_ref())
                            .map(|u| u.eq_ignore_ascii_case("kW"))
                            .unwrap_or(false);
                        power_w = Some(if is_kw { value * 1000.0 } else { value });
                    }
                    MeasurandEnumType::SoC => {
                        soc = Some(value);
                    }
                    _ => {}
                }
            }
        }
    }

    (energy_wh, power_w, soc)
}

/// Build a `TransactionEventResponse` with optional id_token_info.
fn build_response(status: Option<AuthorizationStatusEnumType>) -> Value {
    let response = TransactionEventResponse {
        total_cost: None,
        charging_priority: None,
        id_token_info: status.map(|s| IdTokenInfoType {
            status: s,
            cache_expiry_date_time: None,
            charging_priority: None,
            language1: None,
            evse_id: None,
            language2: None,
            group_id_token: None,
            personal_message: None,
        }),
        updated_personal_message: None,
    };

    serde_json::to_value(&response).unwrap_or_default()
}
