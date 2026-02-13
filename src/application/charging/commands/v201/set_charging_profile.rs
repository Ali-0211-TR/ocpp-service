//! v2.0.1 SetChargingProfile command

use rust_ocpp::v2_0_1::datatypes::charging_profile_type::ChargingProfileType;
use rust_ocpp::v2_0_1::messages::set_charging_profile::{
    SetChargingProfileRequest, SetChargingProfileResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Set a charging profile on a v2.0.1 charging station.
///
/// The `charging_profile` is the full OCPP 2.0.1 `ChargingProfileType`
/// (serialised from the caller). The `evse_id` identifies which EVSE
/// the profile should apply to (0 = entire station).
pub async fn set_charging_profile(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    evse_id: i32,
    charging_profile: ChargingProfileType,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        evse_id,
        profile_id = charging_profile.id,
        "v2.0.1 SetChargingProfile"
    );

    let request = SetChargingProfileRequest {
        evse_id,
        charging_profile,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "SetChargingProfile", payload)
        .await?;

    let response: SetChargingProfileResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
