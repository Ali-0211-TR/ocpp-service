pub mod entities;
pub mod migrator;
pub mod repositories;

pub use repositories::SeaOrmRepositoryProvider;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::info;

use crate::config::DatabasePoolConfig;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database URL (e.g., "sqlite://./ocpp.db?mode=rwc")
    pub url: String,
    /// Connection pool settings
    pub pool: DatabasePoolConfig,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://./ocpp.db?mode=rwc".to_string(),
            pool: DatabasePoolConfig::default(),
        }
    }
}

impl DatabaseConfig {
    /// Create config for SQLite
    pub fn sqlite(path: &str) -> Self {
        Self {
            url: format!("sqlite://{}?mode=rwc", path),
            pool: DatabasePoolConfig::default(),
        }
    }

    /// Create config from environment variable
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://./ocpp.db?mode=rwc".to_string()),
            pool: DatabasePoolConfig::default(),
        }
    }
}

/// Initialize database connection with connection pool settings.
pub async fn init_database(config: &DatabaseConfig) -> Result<DatabaseConnection, sea_orm::DbErr> {
    info!("Connecting to database: {}", config.url);

    let mut opts = ConnectOptions::new(&config.url);
    opts.max_connections(config.pool.max_connections)
        .min_connections(config.pool.min_connections)
        .connect_timeout(std::time::Duration::from_secs(
            config.pool.connect_timeout_seconds,
        ))
        .idle_timeout(std::time::Duration::from_secs(
            config.pool.idle_timeout_seconds,
        ))
        .max_lifetime(std::time::Duration::from_secs(
            config.pool.max_lifetime_seconds,
        ))
        .sqlx_logging(false); // We use tracing instead

    info!(
        "🗄️  DB pool: max={}, min={}, connect_timeout={}s, idle_timeout={}s, max_lifetime={}s",
        config.pool.max_connections,
        config.pool.min_connections,
        config.pool.connect_timeout_seconds,
        config.pool.idle_timeout_seconds,
        config.pool.max_lifetime_seconds,
    );

    let db = Database::connect(opts).await?;
    info!("Database connected successfully");
    Ok(db)
}
