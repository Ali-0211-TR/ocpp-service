//! v2.0.1 GetCompositeSchedule command

use rust_ocpp::v2_0_1::enumerations::charging_rate_unit_enum_type::ChargingRateUnitEnumType;
use rust_ocpp::v2_0_1::messages::get_composite_schedule::{
    GetCompositeScheduleRequest, GetCompositeScheduleResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Result of GetCompositeSchedule (version-agnostic).
pub use super::super::CompositeScheduleResult;

/// Get the composite charging schedule for an EVSE on a v2.0.1 charging station.
///
/// `duration` — requested schedule length in seconds.
/// `charging_rate_unit` — optional `"W"` (Watts) or `"A"` (Amps).
pub async fn get_composite_schedule(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    evse_id: i32,
    duration: i32,
    charging_rate_unit: Option<&str>,
) -> Result<CompositeScheduleResult, CommandError> {
    info!(
        charge_point_id,
        evse_id, duration, "v2.0.1 GetCompositeSchedule"
    );

    let rate_unit = charging_rate_unit.map(|u| match u.to_uppercase().as_str() {
        "A" => ChargingRateUnitEnumType::A,
        _ => ChargingRateUnitEnumType::W,
    });

    let request = GetCompositeScheduleRequest {
        duration,
        charging_rate_unit: rate_unit,
        evse_id,
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
            .schedule
            .map(|s| serde_json::to_value(&s).unwrap_or_default()),
        connector_id: None, // v2.0.1 uses evse_id in request, not returned
        schedule_start: None, // embedded in CompositeScheduleType
    })
}
