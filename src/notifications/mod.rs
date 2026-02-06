//! Notifications module
//!
//! Provides real-time event notifications via WebSocket for UI clients.
//!
//! # Features
//! - Event bus for pub/sub messaging
//! - WebSocket endpoint for UI clients
//! - Filtering by charge point and event type
//!
//! # Usage
//! ```ignore
//! use texnouz_ocpp::notifications::{create_event_bus, Event, ChargePointConnectedEvent};
//! use chrono::Utc;
//!
//! // Create event bus
//! let event_bus = create_event_bus();
//!
//! // Publish events
//! event_bus.publish(Event::ChargePointConnected(ChargePointConnectedEvent {
//!     charge_point_id: "CP001".to_string(),
//!     timestamp: Utc::now(),
//!     remote_addr: Some("192.168.1.100".to_string()),
//! }));
//! ```
//!
//! # WebSocket Endpoint
//! Connect to `/api/v1/notifications/ws` with optional query parameters:
//! - `charge_point_id` - Filter events by charge point
//! - `event_types` - Comma-separated list of event types to receive

pub mod event_bus;
pub mod events;
pub mod websocket;

pub use event_bus::{create_event_bus, EventBus, EventSubscriber, SharedEventBus};
pub use events::*;
pub use websocket::{create_notification_state, ws_notifications_handler, NotificationState};
