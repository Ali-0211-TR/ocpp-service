//! MeterValues handler

use log::{error, info, warn};
use ocpp_rs::v16::call::MeterValues;
use ocpp_rs::v16::call_result::{EmptyResponse, EmptyResponses, ResultPayload};
use ocpp_rs::v16::enums::{Measurand, UnitOfMeasure};

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

    let transaction_id = payload.transaction_id.map(|id| id as i32);

    // Parse all sampled values from all meter values
    let mut energy_wh: Option<f64> = None;
    let mut power_w: Option<f64> = None;
    let mut soc: Option<f64> = None;

    for meter_value in &payload.meter_value {
        for sampled in &meter_value.sampled_value {
            let value: f64 = match sampled.value.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Determine measurand (default is Energy.Active.Import.Register)
            let measurand = sampled.measurand.clone().unwrap_or(Measurand::EnergyActiveImportRegister);

            match measurand {
                Measurand::EnergyActiveImportRegister => {
                    // Convert to Wh if needed
                    let wh = match sampled.unit.as_ref() {
                        Some(UnitOfMeasure::KWh) => value * 1000.0,
                        _ => value, // Default Wh
                    };
                    energy_wh = Some(wh);
                }
                Measurand::PowerActiveImport => {
                    // Convert to W if needed
                    let w = match sampled.unit.as_ref() {
                        Some(UnitOfMeasure::Kw) => value * 1000.0,
                        _ => value, // Default W
                    };
                    power_w = Some(w);
                }
                Measurand::SoC => {
                    soc = Some(value);
                }
                _ => {
                    // Other measurands - log but don't process
                    info!(
                        "[{}] MeterValues - Unhandled measurand: {:?} = {}",
                        handler.charge_point_id, measurand, value
                    );
                }
            }
        }
    }

    info!(
        "[{}] MeterValues parsed - Energy: {:?} Wh, Power: {:?} W, SoC: {:?}%",
        handler.charge_point_id, energy_wh, power_w, soc
    );

    // Update transaction meter data in storage
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
                // Check if charging limit has been reached
                if tx.is_limit_reached() {
                    warn!(
                        "[{}] Charging limit reached for transaction {}! Limit: {:?} = {:?}. Sending RemoteStop.",
                        handler.charge_point_id, tx_id, tx.limit_type, tx.limit_value
                    );

                    // Trigger remote stop
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
                            "[{}] Failed to send RemoteStop for limit-reached transaction {}: {:?}",
                            handler.charge_point_id, tx_id, e
                        );
                    }
                }
            }
            Ok(None) => {
                warn!(
                    "[{}] Transaction {} not found when updating meter data",
                    handler.charge_point_id, tx_id
                );
            }
            Err(e) => {
                error!(
                    "[{}] Failed to update meter data for transaction {}: {:?}",
                    handler.charge_point_id, tx_id, e
                );
            }
        }
    }

    // Calculate energy consumed if we have a transaction
    let energy_consumed_wh = if let (Some(_tx_id), Some(energy)) = (transaction_id, energy_wh) {
        // Try to get meter_start from transaction for consumed calculation
        match handler.service.get_active_transaction_for_connector(
            &handler.charge_point_id,
            payload.connector_id,
        ).await {
            Ok(Some(tx)) => Some(energy - tx.meter_start as f64),
            _ => None,
        }
    } else {
        None
    };

    // Publish event with all parsed values
    handler.event_bus.publish(Event::MeterValuesReceived(MeterValuesEvent {
        charge_point_id: handler.charge_point_id.clone(),
        connector_id: payload.connector_id,
        transaction_id,
        energy_wh,
        energy_consumed_wh,
        power_w,
        soc,
        timestamp: payload.meter_value.first()
            .map(|mv| mv.timestamp.inner())
            .unwrap_or_else(chrono::Utc::now),
    }));

    ResultPayload::PossibleEmptyResponse(EmptyResponses::EmptyResponse(EmptyResponse {}))
}