//! REST API module for OCPP Central System
//!
//! Provides HTTP endpoints for managing charge points, transactions,
//! and sending commands to charging stations.

pub mod dto;
pub mod handlers;
pub mod router;

pub use router::{create_api_router, ApiState};
