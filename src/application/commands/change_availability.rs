//! Change Availability command

use log::info;
use ocpp_rs::v16::call::{Action, ChangeAvailability};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::enums::{AvailabilityType, ParsedGenericStatus};

use super::{CommandError, SharedCommandSender};

/// Availability type for the connector
#[derive(Debug, Clone, Copy)]
pub enum Availability {
    /// Connector is operative
    Operative,
    /// Connector is inoperative
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

/// Change availability of a connector
///
/// Use connector_id = 0 to change availability of the entire charge point
pub async fn change_availability(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    connector_id: u32,
    availability: Availability,
) -> Result<ParsedGenericStatus, CommandError> {
    info!(
        "[{}] ChangeAvailability - ConnectorId: {}, Type: {:?}",
        charge_point_id, connector_id, availability
    );

    let action = Action::ChangeAvailability(ChangeAvailability {
        connector_id,
        availability_type: availability.into(),
    });

    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleStatusResponse(status_response) => {
            Ok(status_response.get_status().clone())
        }
        _ => Err(CommandError::InvalidResponse(
            "Unexpected response type".to_string(),
        )),
    }
}
