//! Domain errors

use std::fmt;

/// Domain-level error types
#[derive(Debug, Clone)]
pub enum DomainError {
    /// Charge point not found
    ChargePointNotFound(String),
    /// Transaction not found
    TransactionNotFound(i32),
    /// Invalid ID tag
    InvalidIdTag(String),
    /// Connector not found
    ConnectorNotFound(u32),
    /// Charge point already exists
    ChargePointAlreadyExists(String),
    /// Transaction already active
    TransactionAlreadyActive(i32),
    /// Authorization failed
    AuthorizationFailed(String),
    /// Storage/database error
    StorageError(String),
    /// Generic error
    Other(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChargePointNotFound(id) => write!(f, "Charge point not found: {}", id),
            Self::TransactionNotFound(id) => write!(f, "Transaction not found: {}", id),
            Self::InvalidIdTag(tag) => write!(f, "Invalid ID tag: {}", tag),
            Self::ConnectorNotFound(id) => write!(f, "Connector not found: {}", id),
            Self::ChargePointAlreadyExists(id) => write!(f, "Charge point already exists: {}", id),
            Self::TransactionAlreadyActive(id) => write!(f, "Transaction already active: {}", id),
            Self::AuthorizationFailed(reason) => write!(f, "Authorization failed: {}", reason),
            Self::StorageError(msg) => write!(f, "Storage error: {}", msg),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for DomainError {}

/// Result type for domain operations
pub type DomainResult<T> = Result<T, DomainError>;
