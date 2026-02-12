//! OCPP protocol shared types
//!
//! Value objects related to the OCPP protocol that don't belong
//! to a single aggregate: protocol versions, API keys, etc.

pub mod api_key;
pub mod version;

pub use api_key::ApiKey;
pub use version::OcppVersion;
