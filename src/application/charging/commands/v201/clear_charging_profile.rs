//! v2.0.1 ClearChargingProfile command

use rust_ocpp::v2_0_1::datatypes::clear_charging_profile_type::ClearChargingProfileType;
use rust_ocpp::v2_0_1::enumerations::charging_profile_purpose_enum_type::ChargingProfilePurposeEnumType;
use rust_ocpp::v2_0_1::messages::clear_charging_profile::{
    ClearChargingProfileRequest, ClearChargingProfileResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Criteria for selecting which charging profiles to clear.
#[derive(Debug, Clone, Default)]
pub struct ClearChargingProfileCriteria {
    /// Clear a specific profile by its ID.
    pub charging_profile_id: Option<i32>,
    /// Restrict to profiles on this EVSE (0 = entire station).
    pub evse_id: Option<i32>,
    /// Restrict to profiles with this purpose.
    pub charging_profile_purpose: Option<String>,
    /// Restrict to profiles at this stack level.
    pub stack_level: Option<i32>,
}

/// Clear charging profiles on a v2.0.1 charging station.
pub async fn clear_charging_profile(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    criteria: ClearChargingProfileCriteria,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        ?criteria,
        "v2.0.1 ClearChargingProfile"
    );

    let purpose = criteria.charging_profile_purpose.as_deref().map(|p| {
        match p {
            "ChargingStationExternalConstraints" => {
                ChargingProfilePurposeEnumType::ChargingStationExternalConstraints
            }
            "ChargingStationMaxProfile" => {
                ChargingProfilePurposeEnumType::ChargingStationMaxProfile
            }
            "TxProfile" => ChargingProfilePurposeEnumType::TxProfile,
            _ => ChargingProfilePurposeEnumType::TxDefaultProfile,
        }
    });

    let charging_profile_criteria =
        if criteria.evse_id.is_some() || purpose.is_some() || criteria.stack_level.is_some() {
            Some(ClearChargingProfileType {
                evse_id: criteria.evse_id,
                charging_profile_purpose: purpose,
                stack_level: criteria.stack_level,
            })
        } else {
            None
        };

    let request = ClearChargingProfileRequest {
        charging_profile_id: criteria.charging_profile_id,
        charging_profile_criteria,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "ClearChargingProfile", payload)
        .await?;

    let response: ClearChargingProfileResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
