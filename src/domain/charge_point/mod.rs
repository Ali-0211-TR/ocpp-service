//! Charge Point aggregate
//!
//! Contains the ChargePoint entity, value objects, and repository interface.

pub mod model;
pub mod repository;

pub use model::{ChargePoint, ChargePointStatus, Connector, ConnectorStatus};
pub use repository::ChargePointRepository;
