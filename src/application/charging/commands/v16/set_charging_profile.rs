//! v1.6 SetChargingProfile command

use rust_ocpp::v1_6::messages::set_charging_profile::{
    SetChargingProfileRequest, SetChargingProfileResponse,
};
use rust_ocpp::v1_6::types::ChargingProfile;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Set a charging profile on a v1.6 charge point.
///
/// The `charging_profile` is the full OCPP 1.6 `ChargingProfile`
/// (deserialized from JSON by the caller).
/// `connector_id` = 0 means the profile applies to the entire charge point.
pub async fn set_charging_profile(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    connector_id: i32,
    charging_profile: ChargingProfile,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        connector_id,
        profile_id = charging_profile.charging_profile_id,
        "v1.6 SetChargingProfile"
    );

    let request = SetChargingProfileRequest {
        connector_id,
        cs_charging_profiles: charging_profile,
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
