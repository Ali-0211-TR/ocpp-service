//! OCPP 2.0.1 protocol adapter
//!
//! Wraps `OcppHandlerV201` behind the `OcppInboundPort` trait,
//! allowing the unified WebSocket server to dispatch OCPP 2.0.1 messages
//! through the version-agnostic adapter architecture.

mod adapter;

pub use adapter::{V201AdapterFactory, V201InboundAdapter};
