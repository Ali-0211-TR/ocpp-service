//! v2.0.1 GetChargingProfiles command

use rust_ocpp::v2_0_1::datatypes::charging_profile_criterion_type::ChargingProfileCriterionType;
use rust_ocpp::v2_0_1::enumerations::charging_profile_purpose_enum_type::ChargingProfilePurposeEnumType;
use rust_ocpp::v2_0_1::messages::get_charging_profiles::{
    GetChargingProfilesRequest, GetChargingProfilesResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Result of a GetChargingProfiles command.
#[derive(Debug, Clone)]
pub struct GetChargingProfilesResult {
    pub status: String,
}

/// Criteria for selecting which charging profiles to retrieve.
#[derive(Debug, Clone, Default)]
pub struct GetChargingProfilesCriteria {
    pub evse_id: Option<i32>,
    pub purpose: Option<String>,
    pub stack_level: Option<i32>,
    pub profile_ids: Option<Vec<i32>>,
}

fn parse_purpose(s: &str) -> Option<ChargingProfilePurposeEnumType> {
    match s {
        "ChargingStationExternalConstraints" => {
            Some(ChargingProfilePurposeEnumType::ChargingStationExternalConstraints)
        }
        "ChargingStationMaxProfile" => {
            Some(ChargingProfilePurposeEnumType::ChargingStationMaxProfile)
        }
        "TxDefaultProfile" => Some(ChargingProfilePurposeEnumType::TxDefaultProfile),
        "TxProfile" => Some(ChargingProfilePurposeEnumType::TxProfile),
        _ => None,
    }
}

pub async fn get_charging_profiles(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    request_id: i32,
    criteria: GetChargingProfilesCriteria,
) -> Result<GetChargingProfilesResult, CommandError> {
    info!(
        charge_point_id,
        request_id,
        evse_id = ?criteria.evse_id,
        purpose = ?criteria.purpose,
        "v2.0.1 GetChargingProfiles"
    );

    let charging_profile = ChargingProfileCriterionType {
        charging_profile_purpose: criteria.purpose.as_deref().and_then(parse_purpose),
        stack_level: criteria.stack_level,
        charging_profile_id: criteria.profile_ids,
        charging_limit_source: None,
    };

    let request = GetChargingProfilesRequest {
        request_id,
        evse_id: criteria.evse_id,
        charging_profile,
    };

    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "GetChargingProfiles", payload)
        .await?;

    let response: GetChargingProfilesResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(GetChargingProfilesResult {
        status: format!("{:?}", response.status),
    })
}
