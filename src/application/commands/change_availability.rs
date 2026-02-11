//! Change Availability command

use ocpp_rs::v16::call::{Action, ChangeAvailability};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::{AvailabilityType, ParsedGenericStatus};
use tracing::info;

use super::{CommandError, SharedCommandSender};

#[derive(Debug, Clone, Copy)]
pub enum Availability {
    Operative,
    Inoperative,
}

impl From<Availability> for AvailabilityType {
    fn from(availability: Availability) -> Self {
        match availability {
            Availability::Operative => AvailabilityType::Operative,
            Availability::Inoperative => AvailabilityType::Inoperative,
        }
    }
}

pub async fn change_availability(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    connector_id: u32,
    availability: Availability,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(charge_point_id, connector_id, ?availability, "ChangeAvailability");

    let action = Action::ChangeAvailability(ChangeAvailability {
        connector_id,
        availability_type: availability.into(),
    });
    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(sr) => Ok(sr.get_status().clone()),
        _ => Err(CommandError::InvalidResponse("Unexpected response type".to_string())),
    }
}
