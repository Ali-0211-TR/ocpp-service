use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Not found: {entity} with {field}={value}")]
    NotFound {
        entity: &'static str,
        field: &'static str,
        value: String,
    },

    #[error("Validation: {0}")]
    Validation(String),

    #[error("Already exists: {0}")]
    Conflict(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Charge point {0} is not connected")]
    ChargePointOffline(String),

    #[error("Command timeout for {0}")]
    CommandTimeout(String),
}

#[derive(Debug, Error)]
pub enum InfraError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error(transparent)]
    Infra(#[from] InfraError),
}