//! OCPP protocol version negotiation
//!
//! During the WebSocket handshake the charge point advertises which OCPP
//! sub-protocols it supports via the `Sec-WebSocket-Protocol` header.
//! The negotiator picks the best mutually-supported version.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::info;

use crate::application::ports::{OcppAdapterFactory, OcppInboundPort};
use crate::domain::OcppVersion;

// ── ProtocolNegotiator ─────────────────────────────────────────

/// Negotiates the OCPP version during WebSocket handshake.
///
/// Picks the highest version that both the charge point and central
/// system support. If no match is found, the connection can be rejected
/// or fall back to a default.
pub struct ProtocolNegotiator {
    /// Versions the CS supports, in preference order (highest first).
    supported_versions: Vec<OcppVersion>,
}

impl ProtocolNegotiator {
    /// Create a negotiator from a list of supported versions.
    pub fn new(supported_versions: Vec<OcppVersion>) -> Self {
        Self { supported_versions }
    }

    /// Negotiate the OCPP version from the `Sec-WebSocket-Protocol` header value.
    ///
    /// Returns the best mutually-supported version, or `None` if no match.
    pub fn negotiate(&self, requested_protocols: &str) -> Option<OcppVersion> {
        let requested: Vec<&str> = requested_protocols.split(',').map(|s| s.trim()).collect();

        // Try our preferred versions in order (highest first)
        for version in &self.supported_versions {
            if requested.iter().any(|p| *p == version.subprotocol()) {
                return Some(*version);
            }
        }

        None
    }

    /// All versions this central system supports.
    pub fn supported_versions(&self) -> &[OcppVersion] {
        &self.supported_versions
    }

    /// Subprotocols to advertise (useful for server info / logging).
    pub fn supported_subprotocols(&self) -> Vec<&'static str> {
        self.supported_versions
            .iter()
            .map(|v| v.subprotocol())
            .collect()
    }
}

// ── ProtocolAdapters ───────────────────────────────────────────

/// Registry that maps OCPP versions to their adapter factories.
///
/// During connection setup the server looks up the negotiated version
/// in this registry to obtain the right factory and create a
/// per-connection inbound adapter.
pub struct ProtocolAdapters {
    factories: HashMap<OcppVersion, Arc<dyn OcppAdapterFactory>>,
}

impl ProtocolAdapters {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register an adapter factory for a specific OCPP version.
    pub fn register(&mut self, version: OcppVersion, factory: Arc<dyn OcppAdapterFactory>) {
        info!(%version, "Registered protocol adapter");
        self.factories.insert(version, factory);
    }

    /// Create a new inbound adapter for the given version and charge point.
    pub fn create_adapter(
        &self,
        version: OcppVersion,
        charge_point_id: String,
    ) -> Option<Box<dyn OcppInboundPort>> {
        self.factories
            .get(&version)
            .map(|f| f.create_inbound_adapter(charge_point_id))
    }

    /// All versions that have a registered adapter.
    pub fn supported_versions(&self) -> Vec<OcppVersion> {
        self.factories.keys().copied().collect()
    }

    /// Build a `ProtocolNegotiator` from the registered versions.
    ///
    /// Versions are ordered highest-first so the negotiator prefers
    /// the newest mutually-supported protocol.
    pub fn build_negotiator(&self) -> ProtocolNegotiator {
        let mut versions = self.supported_versions();
        versions.sort_by(|a, b| {
            // Sort by version descending (V21 > V201 > V16)
            let order = |v: &OcppVersion| match v {
                OcppVersion::V21 => 3,
                OcppVersion::V201 => 2,
                OcppVersion::V16 => 1,
            };
            order(b).cmp(&order(a))
        });
        ProtocolNegotiator::new(versions)
    }
}

impl Default for ProtocolAdapters {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiate_single_match() {
        let negotiator = ProtocolNegotiator::new(vec![OcppVersion::V16]);
        assert_eq!(negotiator.negotiate("ocpp1.6"), Some(OcppVersion::V16));
    }

    #[test]
    fn negotiate_multiple_prefers_highest() {
        let negotiator =
            ProtocolNegotiator::new(vec![OcppVersion::V21, OcppVersion::V201, OcppVersion::V16]);
        // CP supports both 1.6 and 2.0.1 — CS should pick 2.0.1 (highest mutual)
        assert_eq!(
            negotiator.negotiate("ocpp1.6, ocpp2.0.1"),
            Some(OcppVersion::V201)
        );
    }

    #[test]
    fn negotiate_no_match() {
        let negotiator = ProtocolNegotiator::new(vec![OcppVersion::V16]);
        assert_eq!(negotiator.negotiate("ocpp2.0.1"), None);
    }

    #[test]
    fn negotiate_empty_header() {
        let negotiator = ProtocolNegotiator::new(vec![OcppVersion::V16]);
        assert_eq!(negotiator.negotiate(""), None);
    }
}
