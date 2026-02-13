//! v1.6 CancelReservation command

use rust_ocpp::v1_6::messages::cancel_reservation::{
    CancelReservationRequest, CancelReservationResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

pub async fn cancel_reservation(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    reservation_id: i32,
) -> Result<String, CommandError> {
    info!(
        charge_point_id,
        reservation_id,
        "v1.6 CancelReservation"
    );

    let request = CancelReservationRequest { reservation_id };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "CancelReservation", payload)
        .await?;

    let response: CancelReservationResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(format!("{:?}", response.status))
}
