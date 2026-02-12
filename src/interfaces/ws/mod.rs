//! WebSocket interfaces
//!
//! - `ocpp_server`: Unified OCPP WebSocket server (multi-version)
//! - `negotiator`: Protocol version negotiation and adapter registry
//! - `ocpp_v16`: OCPP 1.6 protocol adapter
//! - `ocpp_v201`: OCPP 2.0.1 protocol adapter
//! - `notifications`: Real-time event streaming to UI clients

pub mod negotiator;
pub mod notifications;
pub mod ocpp_server;
pub mod ocpp_v16;
pub mod ocpp_v201;

pub use negotiator::ProtocolAdapters;
pub use notifications::{create_notification_state, ws_notifications_handler, NotificationState};
pub use ocpp_server::OcppServer;
pub use ocpp_v16::V16AdapterFactory;
pub use ocpp_v201::V201AdapterFactory;
