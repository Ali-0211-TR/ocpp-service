//! Domain events
//!
//! Event types that represent facts about what happened in the system.
//! The EventBus implementation lives in `application::events`.

pub mod types;

// Re-export all event types
pub use types::{
    AuthorizationEvent, BootNotificationEvent, ChargePointConnectedEvent,
    ChargePointDisconnectedEvent, ChargePointStatusChangedEvent, ConnectorStatusChangedEvent,
    ErrorEvent, Event, EventMessage, HeartbeatEvent, MeterValuesEvent, TransactionStartedEvent,
    TransactionStoppedEvent,
};
