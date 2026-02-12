//! Inbound ports — interfaces for handling incoming OCPP messages
//!
//! Each OCPP version provides an adapter that implements `OcppInboundPort`
//! to handle charge-point → central-system messages.

use std::fmt;

use async_trait::async_trait;

use crate::domain::OcppVersion;

// ── OcppInboundPort ────────────────────────────────────────────

/// Port for handling inbound OCPP messages from a charge point.
///
/// An adapter is created **per connection** and handles all messages
/// for a single charge point during the lifetime of that connection.
///
/// The adapter is responsible for:
/// - Parsing version-specific action payloads
/// - Routing to the appropriate business-logic handler
/// - Serializing version-specific response payloads
#[async_trait]
pub trait OcppInboundPort: Send + Sync {
    /// Handle a raw OCPP text message and return an optional response.
    ///
    /// The adapter internally parses the OCPP-J frame, dispatches to
    /// version-specific handlers, and serializes the response.
    async fn handle_message(&self, text: &str) -> Option<String>;

    /// The OCPP version this adapter handles.
    fn version(&self) -> OcppVersion;

    /// The charge point ID this adapter is associated with.
    fn charge_point_id(&self) -> &str;
}

// ── OcppAdapterFactory ─────────────────────────────────────────

/// Factory for creating per-connection inbound adapters.
///
/// One factory is registered per OCPP version. When a new charge point
/// connects and negotiates a version, the corresponding factory creates
/// an adapter instance for that connection.
pub trait OcppAdapterFactory: Send + Sync {
    /// Create a new inbound adapter for the given charge point.
    fn create_inbound_adapter(&self, charge_point_id: String) -> Box<dyn OcppInboundPort>;

    /// The OCPP version this factory creates adapters for.
    fn version(&self) -> OcppVersion;
}

// ── ProtocolError ──────────────────────────────────────────────

/// Errors that can occur during protocol-level processing.
#[derive(Debug)]
pub enum ProtocolError {
    /// Unknown or unsupported action name.
    UnknownAction(String),
    /// Failed to deserialize a message payload.
    DeserializationError(String),
    /// Failed to serialize a response payload.
    SerializationError(String),
    /// Internal handler error.
    InternalError(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAction(a) => write!(f, "Unknown action: {}", a),
            Self::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ProtocolError {}
