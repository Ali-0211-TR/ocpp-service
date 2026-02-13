//! v1.6 ClearChargingProfile command

use rust_ocpp::v1_6::messages::clear_charging_profile::{
    ClearChargingProfileRequest, ClearChargingProfileResponse,
};
use rust_ocpp::v1_6::types::ChargingProfilePurposeType;
use tracing::info;

use crate::application::charging::commands::dispatcher::ClearChargingProfileCriteria;
use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Clear charging profiles on a v1.6 charge point.
pub async fn clear_charging_profile(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    criteria: ClearChargingProfileCriteria,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        ?criteria,
        "v1.6 ClearChargingProfile"
    );

    let purpose = criteria.charging_profile_purpose.as_deref().map(|p| match p {
        "ChargePointMaxProfile" => ChargingProfilePurposeType::ChargePointMaxProfile,
        "TxDefaultProfile" => ChargingProfilePurposeType::TxDefaultProfile,
        "TxProfile" => ChargingProfilePurposeType::TxProfile,
        // v2.0.1 names → v1.6 equivalents
        "ChargingStationMaxProfile" => ChargingProfilePurposeType::ChargePointMaxProfile,
        _ => ChargingProfilePurposeType::TxDefaultProfile,
    });

    let request = ClearChargingProfileRequest {
        id: criteria.charging_profile_id,
        connector_id: criteria.evse_id,
        charging_profile_purpose: purpose,
        stack_level: criteria.stack_level,
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
