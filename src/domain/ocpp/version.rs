//! OCPP protocol version
//!
//! Defines the supported OCPP versions for multi-protocol support.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Supported OCPP protocol versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OcppVersion {
    /// OCPP 1.6 (JSON / OCPP-J)
    V16,
    /// OCPP 2.0.1
    V201,
    /// OCPP 2.1
    V21,
}

impl OcppVersion {
    /// WebSocket subprotocol identifier for this OCPP version.
    ///
    /// Used in the `Sec-WebSocket-Protocol` header during handshake.
    pub fn subprotocol(&self) -> &'static str {
        match self {
            Self::V16 => "ocpp1.6",
            Self::V201 => "ocpp2.0.1",
            Self::V21 => "ocpp2.1",
        }
    }

    /// Parse an OCPP version from a WebSocket subprotocol string.
    pub fn from_subprotocol(s: &str) -> Option<Self> {
        match s.trim() {
            "ocpp1.6" => Some(Self::V16),
            "ocpp2.0.1" => Some(Self::V201),
            "ocpp2.1" => Some(Self::V21),
            _ => None,
        }
    }

    /// All supported OCPP versions, ordered from newest to oldest.
    pub const ALL: &'static [OcppVersion] = &[Self::V21, Self::V201, Self::V16];

    /// Human-readable version string.
    pub fn version_string(&self) -> &'static str {
        match self {
            Self::V16 => "1.6",
            Self::V201 => "2.0.1",
            Self::V21 => "2.1",
        }
    }
}

impl fmt::Display for OcppVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OCPP {}", self.version_string())
    }
}
