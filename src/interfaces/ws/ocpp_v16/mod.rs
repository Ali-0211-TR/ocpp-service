//! OCPP 1.6 protocol adapter
//!
//! Wraps the existing `OcppHandler` behind the `OcppInboundPort` trait,
//! allowing the unified WebSocket server to dispatch OCPP 1.6 messages
//! through the version-agnostic adapter architecture.

mod adapter;

pub use adapter::{V16AdapterFactory, V16InboundAdapter};
