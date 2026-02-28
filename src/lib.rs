//! # Texnouz CSMS
//!
//! OCPP 1.6 Central System implementation for managing EV charging stations.
//!
//! ## Architecture (Clean / SOLID)
//!
//! - **support**: Cross-cutting utilities (errors, shutdown, time, ID generation)
//! - **domain**: Core business entities, traits, and value objects
//! - **application**: Use-case orchestration, commands, events, DTOs
//! - **infrastructure**: External concerns (database, crypto)
//! - **interfaces**: Delivery mechanisms (HTTP REST, WebSocket, gRPC placeholder)
//! - **config**: Application configuration (TOML-based)

pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod interfaces;
pub mod server;
pub mod shared;

// Re-export commonly used types at crate root
pub use application::events::{create_event_bus, Event, EventBus, SharedEventBus};
pub use config::{default_config_path, AppConfig, Config};
pub use infrastructure::{init_database, DatabaseConfig, SeaOrmRepositoryProvider};
pub use interfaces::http::create_api_router;
