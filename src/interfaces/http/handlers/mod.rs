//! HTTP API handlers

pub mod api_keys;
pub mod auth;
pub mod charge_points;
pub mod commands;
pub mod health;
pub mod id_tags;
pub mod monitoring;
pub mod tariffs;
pub mod transactions;

// Re-export handler state types for router use
pub use charge_points::AppState;
pub use commands::CommandAppState;
pub use transactions::TransactionAppState;
pub use monitoring::MonitoringState;
pub use auth::AuthHandlerState;
pub use api_keys::ApiKeyHandlerState;
pub use id_tags::IdTagHandlerState;
