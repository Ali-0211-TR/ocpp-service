//! v1.6 GetCompositeSchedule command

use rust_ocpp::v1_6::messages::get_composite_schedule::{
    GetCompositeScheduleRequest, GetCompositeScheduleResponse,
};
use rust_ocpp::v1_6::types::ChargingRateUnitType;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Result of GetCompositeSchedule (version-agnostic).
pub use super::super::CompositeScheduleResult;

/// Get the composite charging schedule for a connector on a v1.6 charge point.
///
/// `duration` — requested schedule length in seconds.
/// `charging_rate_unit` — optional `"W"` (Watts) or `"A"` (Amps).
pub async fn get_composite_schedule(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    connector_id: i32,
    duration: i32,
    charging_rate_unit: Option<&str>,
) -> Result<CompositeScheduleResult, CommandError> {
    info!(
        charge_point_id,
        connector_id, duration, "v1.6 GetCompositeSchedule"
    );

    let rate_unit = charging_rate_unit.map(|u| match u.to_uppercase().as_str() {
        "A" => ChargingRateUnitType::A,
        _ => ChargingRateUnitType::W,
    });

    let request = GetCompositeScheduleRequest {
        connector_id,
        duration,
        charging_rate_unit: rate_unit,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "GetCompositeSchedule", payload)
        .await?;

    let response: GetCompositeScheduleResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(CompositeScheduleResult {
        status: format!("{:?}", response.status),
        schedule: response
            .charging_schedule
            .map(|s| serde_json::to_value(&s).unwrap_or_default()),
        connector_id: response.connector_id,
        schedule_start: response.schedule_start.map(|dt| dt.to_rfc3339()),
    })
}
