//! V201 MeterValues handler
//!
//! In OCPP 2.0.1, MeterValues uses `evse_id` instead of `connector_id`,
//! and `SampledValueType.value` is `Decimal` (not `String`).

use rust_ocpp::v2_0_1::enumerations::measurand_enum_type::MeasurandEnumType;
use rust_ocpp::v2_0_1::messages::meter_values::{MeterValuesRequest, MeterValuesResponse};
use serde_json::Value;
use tracing::{error, info, warn};

use crate::application::events::{Event, MeterValuesEvent};
use crate::application::OcppHandlerV201;

pub async fn handle_meter_values(handler: &OcppHandlerV201, payload: &Value) -> Value {
    let req: MeterValuesRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse MeterValues"
            );
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        evse_id = req.evse_id,
        samples = req.meter_value.len(),
        "V201 MeterValues"
    );

    let evse_id = req.evse_id as u32;
    let mut energy_wh: Option<f64> = None;
    let mut power_w: Option<f64> = None;
    let mut soc: Option<f64> = None;

    for meter_value in &req.meter_value {
        for sampled in &meter_value.sampled_value {
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
                _ => {
                    info!(
                        charge_point_id = handler.charge_point_id.as_str(),
                        ?measurand,
                        value,
                        "V201: Unhandled measurand"
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
        "V201 MeterValues parsed"
    );

    // Find active transaction for this EVSE and update meter data
    let mut transaction_id: Option<i32> = None;
    let mut energy_consumed_wh: Option<f64> = None;

    match handler
        .service
        .get_active_transaction_for_connector(&handler.charge_point_id, evse_id)
        .await
    {
        Ok(Some(tx)) => {
            transaction_id = Some(tx.id);

            let _ = handler
                .service
                .update_transaction_meter_data(
                    tx.id,
                    energy_wh.map(|e| e as i32),
                    power_w,
                    soc.map(|s| s as i32),
                )
                .await;

            if let Some(energy) = energy_wh {
                energy_consumed_wh = Some(energy - tx.meter_start as f64);
            }
        }
        Ok(None) => {
            warn!(
                charge_point_id = handler.charge_point_id.as_str(),
                evse_id, "V201: No active transaction found for MeterValues"
            );
        }
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                evse_id,
                error = ?e,
                "V201: Failed to look up transaction for MeterValues"
            );
        }
    }

    handler
        .event_bus
        .publish(Event::MeterValuesReceived(MeterValuesEvent {
            charge_point_id: handler.charge_point_id.clone(),
            connector_id: evse_id,
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
        }));

    serde_json::to_value(&MeterValuesResponse {}).unwrap_or_default()
}
