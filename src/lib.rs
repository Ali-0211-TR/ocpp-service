//! # Texnouz OCPP Central System
//!
//! OCPP 1.6 Central System implementation for managing EV charging stations.
//!
//! ## Architecture
//!
//! The project follows Clean Architecture principles:
//!
//! - **domain**: Core business entities, types and traits
//! - **application**: Business logic, use cases and handlers
//! - **infrastructure**: External concerns (WebSocket server, storage, database)
//! - **session**: Connection and session management
//! - **api**: REST API with Swagger documentation
//! - **auth**: JWT authentication and API key management
//! - **notifications**: Real-time WebSocket notifications for UI

pub mod api;
pub mod application;
pub mod auth;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod notifications;
pub mod session;

pub use config::{AppConfig, Config, default_config_path};

// Re-export database types for easy access
pub use infrastructure::{init_database, DatabaseConfig, DatabaseStorage};

// Re-export API router
pub use api::create_api_router;

// Re-export notifications
pub use notifications::{create_event_bus, Event, EventBus, SharedEventBus};
