//! Application events (pub/sub)
//!
//! Event types are defined in `domain::events`. The `EventBus`
//! implementation (broadcast channel) lives here in the application layer.

pub mod event_bus;

// Re-export domain event types for backward compatibility
pub use crate::domain::events::types;
pub use crate::domain::events::types::*;

pub use event_bus::{create_event_bus, EventBus, EventSubscriber, SharedEventBus};
