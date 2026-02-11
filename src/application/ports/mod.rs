//! Application ports (hexagonal architecture boundaries)
//!
//! Ports define the interfaces between the application core and the outside world.
//! Protocol-specific adapters implement these traits to translate between
//! wire formats (OCPP 1.6, 2.0.1, 2.1) and the version-agnostic application layer.

pub mod inbound;
pub mod outbound;

pub use inbound::{OcppAdapterFactory, OcppInboundPort, ProtocolError};
pub use outbound::OcppOutboundPort;
