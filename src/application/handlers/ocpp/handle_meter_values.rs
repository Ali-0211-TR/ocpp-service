//! MeterValues handler

use ocpp_rs::v16::call::MeterValues;
use ocpp_rs::v16::call_result::{EmptyResponse, EmptyResponses, ResultPayload};
use ocpp_rs::v16::enums::{Measurand, UnitOfMeasure};
use tracing::{error, info, warn};

use crate::application::events::{Event, MeterValuesEvent};
use crate::application::OcppHandler;

pub async fn handle_meter_values(handler: &OcppHandler, payload: MeterValues) -> ResultPayload {
    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        connector_id = payload.connector_id,
        transaction_id = ?payload.transaction_id,
        samples = payload.meter_value.len(),
        "MeterValues"
    );

    let transaction_id = payload.transaction_id.map(|id| id as i32);

    let mut energy_wh: Option<f64> = None;
    let mut power_w: Option<f64> = None;
    let mut soc: Option<f64> = None;

    for meter_value in &payload.meter_value {
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
                if tx.is_limit_reached() {
                    warn!(
                        charge_point_id = handler.charge_point_id.as_str(),
                        transaction_id = tx_id,
                        limit_type = ?tx.limit_type,
                        limit_value = ?tx.limit_value,
                        "Charging limit reached! Sending RemoteStop."
                    );

                    let action = ocpp_rs::v16::call::Action::RemoteStopTransaction(
                        ocpp_rs::v16::call::RemoteStopTransaction {
                            transaction_id: tx_id,
                        },
                    );
                    if let Err(e) = handler
                        .command_sender
                        .send_command(&handler.charge_point_id, action)
                        .await
                    {
                        error!(
                            charge_point_id = handler.charge_point_id.as_str(),
                            transaction_id = tx_id,
                            error = ?e,
                            "Failed to send RemoteStop for limit-reached transaction"
                        );
                    }
                }
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

    let energy_consumed_wh = if let (Some(_tx_id), Some(energy)) = (transaction_id, energy_wh) {
        match handler
            .service
            .get_active_transaction_for_connector(
                &handler.charge_point_id,
                payload.connector_id,
            )
            .await
        {
            Ok(Some(tx)) => Some(energy - tx.meter_start as f64),
            _ => None,
        }
    } else {
        None
    };

    handler.event_bus.publish(Event::MeterValuesReceived(MeterValuesEvent {
        charge_point_id: handler.charge_point_id.clone(),
        connector_id: payload.connector_id,
        transaction_id,
        energy_wh,
        energy_consumed_wh,
        power_w,
        soc,
        timestamp: payload
            .meter_value
            .first()
            .map(|mv| mv.timestamp.inner())
            .unwrap_or_else(chrono::Utc::now),
    }));

    ResultPayload::PossibleEmptyResponse(EmptyResponses::EmptyResponse(EmptyResponse {}))
}
