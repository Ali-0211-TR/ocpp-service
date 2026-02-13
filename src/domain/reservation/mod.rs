//! Reservation aggregate
//!
//! Contains the Reservation entity, related types, and repository interface.

pub mod model;
pub mod repository;

pub use model::{Reservation, ReservationStatus};
pub use repository::ReservationRepository;
