//! WebSocket interfaces
//!
//! - `ocpp_server`: OCPP 1.6 charge-point connections
//! - `notifications`: Real-time event streaming to UI clients

pub mod notifications;
pub mod ocpp_server;

pub use notifications::{
    create_notification_state, ws_notifications_handler, NotificationState,
};
pub use ocpp_server::OcppServer;