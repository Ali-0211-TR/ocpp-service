//! Application ports (hexagonal architecture boundaries)
//!
//! Inbound ports (domain contracts) are defined in `domain::ports`.
//! Outbound ports that depend on application-layer types live here.

pub mod outbound;

// Re-export domain inbound ports for backward compatibility
pub use crate::domain::ports::inbound;
pub use crate::domain::ports::inbound::{OcppAdapterFactory, OcppInboundPort, ProtocolError};
pub use outbound::OcppOutboundPort;
