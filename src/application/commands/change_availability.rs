//! Change Availability command

use rust_ocpp::v1_6::messages::change_availability::{
    ChangeAvailabilityRequest, ChangeAvailabilityResponse,
};
use rust_ocpp::v1_6::types::AvailabilityType;
use tracing::info;

use super::{CommandError, SharedCommandSender};

#[derive(Debug, Clone, Copy)]
pub enum Availability {
    Operative,
    Inoperative,
}

pub async fn change_availability(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    connector_id: u32,
    availability: Availability,
) -> Result<String, CommandError> {
    info!(charge_point_id, connector_id, ?availability, "ChangeAvailability");

    let kind = match availability {
        Availability::Operative => AvailabilityType::Operative,
        Availability::Inoperative => AvailabilityType::Inoperative,
    };

    let request = ChangeAvailabilityRequest {
        connector_id,
        kind,
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
