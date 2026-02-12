//! Domain ports (hexagonal architecture boundaries)
//!
//! Ports define the interfaces between the domain core and the outside world.
//! These are trait contracts that external adapters implement.

pub mod inbound;

pub use inbound::{OcppAdapterFactory, OcppInboundPort, ProtocolError};
