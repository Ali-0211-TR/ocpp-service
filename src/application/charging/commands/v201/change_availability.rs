//! v2.0.1 ChangeAvailability command

use rust_ocpp::v2_0_1::messages::change_availability::{
    ChangeAvailabilityRequest, ChangeAvailabilityResponse,
};
use rust_ocpp::v2_0_1::datatypes::evse_type::EVSEType;
use rust_ocpp::v2_0_1::enumerations::operational_status_enum_type::OperationalStatusEnumType;
use tracing::info;

use crate::application::charging::commands::{Availability, CommandError, SharedCommandSender};

/// In v2.0.1 the identifier is `evse_id` (EVSE-based) rather than `connector_id`.
/// When `evse_id` is 0, availability applies to the entire Charging Station.
pub async fn change_availability(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    evse_id: i32,
    connector_id: Option<i32>,
    availability: Availability,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        evse_id,
        ?connector_id,
        ?availability,
        "v2.0.1 ChangeAvailability"
    );

    let operational_status = match availability {
        Availability::Operative => OperationalStatusEnumType::Operative,
        Availability::Inoperative => OperationalStatusEnumType::Inoperative,
    };

    let evse = if evse_id > 0 {
        Some(EVSEType {
            id: evse_id,
            connector_id,
        })
    } else {
        None // Applies to entire station
    };

    let request = ChangeAvailabilityRequest {
        operational_status,
        evse,
    };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "ChangeAvailability", payload)
        .await?;

    let response: ChangeAvailabilityResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
