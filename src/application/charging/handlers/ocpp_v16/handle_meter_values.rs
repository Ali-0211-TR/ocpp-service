//! MeterValues handler

use rust_ocpp::v1_6::messages::meter_values::{MeterValuesRequest, MeterValuesResponse};
use rust_ocpp::v1_6::types::{Measurand, UnitOfMeasure};
use serde_json::Value;
use tracing::{error, info, warn};

use crate::application::events::{Event, MeterValuesEvent};
use crate::application::OcppHandlerV16;

pub async fn handle_meter_values(handler: &OcppHandlerV16, payload: &Value) -> Value {
    let req: MeterValuesRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(charge_point_id = handler.charge_point_id.as_str(), error = %e, "Failed to parse MeterValues");
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        connector_id = req.connector_id,
        transaction_id = ?req.transaction_id,
        samples = req.meter_value.len(),
        "MeterValues"
    );

    let transaction_id = req.transaction_id;

    let mut energy_wh: Option<f64> = None;
    let mut power_w: Option<f64> = None;
    let mut soc: Option<f64> = None;

    for meter_value in &req.meter_value {
        for sampled in &meter_value.sampled_value {
            let value: f64 = match sampled.value.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let measurand = sampled
                .measurand
                .clone()
                .unwrap_or(Measurand::EnergyActiveImportRegister);

            match measurand {
                Measurand::EnergyActiveImportRegister => {
                    let wh = match sampled.unit.as_ref() {
                        Some(UnitOfMeasure::KWh) => value * 1000.0,
                        _ => value,
                    };
                    energy_wh = Some(wh);
                }
                Measurand::PowerActiveImport => {
                    let w = match sampled.unit.as_ref() {
                        Some(UnitOfMeasure::Kw) => value * 1000.0,
                        _ => value,
                    };
                    power_w = Some(w);
                }
                Measurand::SoC => {
                    soc = Some(value);
                }
                _ => {
                    info!(
                        charge_point_id = handler.charge_point_id.as_str(),
                        ?measurand,
                        value,
                        "Unhandled measurand"
                    );
                }
            }
        }
    }

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        ?energy_wh,
        ?power_w,
        ?soc,
        "MeterValues parsed"
    );

    // Track the transaction found via transactionId (if present) so we can
    // re-use it for energy_consumed / external_order_id computation below
    // without a second DB lookup that may fail if status != "Active".
    let mut tx_from_meter_update: Option<crate::domain::transaction::Transaction> = None;

    if let Some(tx_id) = transaction_id {
        match handler
            .service
            .update_transaction_meter_data(
                tx_id,
                energy_wh.map(|e| e as i32),
                power_w,
                soc.map(|s| s as i32),
            )
            .await
        {
            Ok(Some(tx)) => {
                if tx.is_limit_reached() && handler.service.mark_stop_sent(tx_id) {
                    warn!(
                        charge_point_id = handler.charge_point_id.as_str(),
                        transaction_id = tx_id,
                        limit_type = ?tx.limit_type,
                        limit_value = ?tx.limit_value,
                        "Charging limit reached! Sending RemoteStop."
                    );

                    let cmd = handler.command_sender.clone();
                    let cp_id = handler.charge_point_id.clone();
                    tokio::spawn(async move {
                        let remote_stop_payload = serde_json::json!({
                            "transactionId": tx_id,
                        });
                        if let Err(e) = cmd
                            .send_command(
                                &cp_id,
                                "RemoteStopTransaction",
                                remote_stop_payload,
                            )
                            .await
                        {
                            error!(
                                charge_point_id = cp_id.as_str(),
                                transaction_id = tx_id,
                                error = ?e,
                                "Failed to send RemoteStop for limit-reached transaction"
                            );
                        }
                    });
                }
                tx_from_meter_update = Some(tx);
            }
            Ok(None) => {
                warn!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    transaction_id = tx_id,
                    "Transaction not found when updating meter data"
                );
            }
            Err(e) => {
                error!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    transaction_id = tx_id,
                    error = ?e,
                    "Failed to update meter data"
                );
            }
        }
    }

    // Compute energy consumed in this session and resolve external_order_id.
    // 1. Prefer the transaction already fetched above (via transactionId).
    // 2. Fall back to connector-based lookup when transactionId is absent.
    let (energy_consumed_wh, external_order_id) = if let Some(energy) = energy_wh {
        // Try the transaction we already have from update_transaction_meter_data
        if let Some(ref tx) = tx_from_meter_update {
            (Some(energy - tx.meter_start as f64), tx.external_order_id.clone())
        } else {
            // Fallback: look up by connector (for chargers that omit transactionId)
            match handler
                .service
                .get_active_transaction_for_connector(&handler.charge_point_id, req.connector_id)
                .await
            {
                Ok(Some(tx)) => {
                    // If transactionId was absent, we skipped the auto-stop check above.
                    // Perform it here using the looked-up active transaction.
                    if transaction_id.is_none() {
                        if let Ok(Some(updated_tx)) = handler
                            .service
                            .update_transaction_meter_data(
                                tx.id,
                                energy_wh.map(|e| e as i32),
                                power_w,
                                soc.map(|s| s as i32),
                            )
                            .await
                        {
                            if updated_tx.is_limit_reached() && handler.service.mark_stop_sent(tx.id) {
                                warn!(
                                    charge_point_id = handler.charge_point_id.as_str(),
                                    transaction_id = tx.id,
                                    limit_type = ?updated_tx.limit_type,
                                    limit_value = ?updated_tx.limit_value,
                                    "Charging limit reached (no txId in MeterValues)! Sending RemoteStop."
                                );

                                let cmd = handler.command_sender.clone();
                                let cp_id = handler.charge_point_id.clone();
                                let t_id = tx.id;
                                tokio::spawn(async move {
                                    let remote_stop_payload = serde_json::json!({
                                        "transactionId": t_id,
                                    });
                                    if let Err(e) = cmd
                                        .send_command(
                                            &cp_id,
                                            "RemoteStopTransaction",
                                            remote_stop_payload,
                                        )
                                        .await
                                    {
                                        error!(
                                            charge_point_id = cp_id.as_str(),
                                            transaction_id = t_id,
                                            error = ?e,
                                            "Failed to send RemoteStop for limit-reached transaction (fallback)"
                                        );
                                    }
                                });
                            }
                        }
                    }

                    (Some(energy - tx.meter_start as f64), tx.external_order_id.clone())
                }
                _ => (None, None),
            }
        }
    } else {
        (None, None)
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        ?energy_consumed_wh,
        ?external_order_id,
        "MeterValues notification"
    );

    handler
        .event_bus
        .publish(Event::MeterValuesReceived(MeterValuesEvent {
            charge_point_id: handler.charge_point_id.clone(),
            connector_id: req.connector_id,
            transaction_id,
            energy_wh,
            energy_consumed_wh,
            power_w,
            soc,
            timestamp: req
                .meter_value
                .first()
                .map(|mv| mv.timestamp)
                .unwrap_or_else(chrono::Utc::now),
            external_order_id,
        }));

    serde_json::to_value(&MeterValuesResponse {}).unwrap_or_default()
}
