//! HTTP REST API interfaces
//!
//! - `middleware`: Authentication middleware (JWT + API key)
//! - `handlers`: Request handlers for all resources
//! - `router`: API router with Swagger documentation

pub mod handlers;
pub mod middleware;
pub mod router;
pub mod dto;

pub use router::create_api_router;
pub use dto::*;