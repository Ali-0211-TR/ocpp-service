pub mod entities;
pub mod migrator;
pub mod repositories;

pub mod storage;

pub use storage::DatabaseStorage;

use tracing::info;
use sea_orm::{Database, DatabaseConnection};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database URL (e.g., "sqlite://./ocpp.db?mode=rwc")
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://./ocpp.db?mode=rwc".to_string(),
        }
    }
}

impl DatabaseConfig {
    /// Create config for SQLite
    pub fn sqlite(path: &str) -> Self {
        Self {
            url: format!("sqlite://{}?mode=rwc", path),
        }
    }

    /// Create config from environment variable
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://./ocpp.db?mode=rwc".to_string()),
        }
    }
}

/// Initialize database connection
pub async fn init_database(config: &DatabaseConfig) -> Result<DatabaseConnection, sea_orm::DbErr> {
    info!("Connecting to database: {}", config.url);
    let db = Database::connect(&config.url).await?;
    info!("Database connected successfully");
    Ok(db)
}