//! Reservation DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request to create a new reservation (ReserveNow)
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateReservationRequest {
    /// Charge point ID to reserve on
    pub charge_point_id: String,
    /// Connector ID (0 = any connector)
    #[serde(default)]
    pub connector_id: i32,
    /// ID tag (RFID) for this reservation
    pub id_tag: String,
    /// Optional parent ID tag (group)
    pub parent_id_tag: Option<String>,
    /// Reservation expiry date (ISO 8601)
    pub expiry_date: String,
}

/// Reservation details in API responses
#[derive(Debug, Serialize, ToSchema)]
pub struct ReservationDto {
    pub id: i32,
    pub charge_point_id: String,
    pub connector_id: i32,
    pub id_tag: String,
    pub parent_id_tag: Option<String>,
    pub expiry_date: String,
    pub status: String,
    pub created_at: String,
}

/// Response from creating a reservation
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateReservationResponse {
    pub reservation_id: i32,
    /// Status returned by the charge point (e.g. "Accepted", "Rejected", "Occupied")
    pub status: String,
    pub message: Option<String>,
}

/// Response from cancelling a reservation
#[derive(Debug, Serialize, ToSchema)]
pub struct CancelReservationResponse {
    /// Status returned by the charge point ("Accepted" or "Rejected")
    pub status: String,
    pub message: Option<String>,
}
