//! Configuration module
//!
//! TOML-based persistent configuration with auto-creation and defaults.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Root application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// General server settings
    #[serde(default)]
    pub server: ServerConfig,

    /// Database settings
    #[serde(default)]
    pub database: DatabaseSettings,

    /// JWT / security settings
    #[serde(default)]
    pub security: SecurityConfig,

    /// Admin account (first launch)
    #[serde(default)]
    pub admin: AdminConfig,

    /// Logging
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// WebSocket + REST server settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// REST API bind host
    #[serde(default = "default_host")]
    pub api_host: String,

    /// REST API port
    #[serde(default = "default_api_port")]
    pub api_port: u16,

    /// OCPP WebSocket bind host
    #[serde(default = "default_host")]
    pub ws_host: String,

    /// OCPP WebSocket port
    #[serde(default = "default_ws_port")]
    pub ws_port: u16,

    /// Heartbeat interval sent to charge points (seconds)
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: i32,

    /// Graceful shutdown timeout (seconds)
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout: u64,
}

/// Database type selector
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DbType {
    Sqlite,
    Postgres,
}

/// Database settings with driver selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    /// Which database backend to use
    #[serde(default = "default_db_type")]
    pub driver: DbType,

    /// SQLite settings (used when driver = "sqlite")
    #[serde(default)]
    pub sqlite: SqliteConfig,

    /// PostgreSQL settings (used when driver = "postgres")
    #[serde(default)]
    pub postgres: PostgresConfig,
}

/// SQLite-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// Path to the database file
    #[serde(default = "default_sqlite_path")]
    pub path: String,
}

/// PostgreSQL-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Hostname
    #[serde(default = "default_pg_host")]
    pub host: String,

    /// Port
    #[serde(default = "default_pg_port")]
    pub port: u16,

    /// Username
    #[serde(default = "default_pg_user")]
    pub username: String,

    /// Password
    #[serde(default)]
    pub password: String,

    /// Database name
    #[serde(default = "default_pg_database")]
    pub database: String,
}

/// JWT and other security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT signing secret
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,

    /// Token lifetime in hours
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration_hours: i64,
}

/// Default admin account created on first launch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    /// Admin username
    #[serde(default = "default_admin_username")]
    pub username: String,

    /// Admin email
    #[serde(default = "default_admin_email")]
    pub email: String,

    /// Admin password (used only for initial creation)
    #[serde(default = "default_admin_password")]
    pub password: String,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level: error, warn, info, debug, trace
    #[serde(default = "default_log_level")]
    pub level: String,
}

// ── Default value helpers ──────────────────────────────────────

fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_api_port() -> u16 {
    8080
}
fn default_ws_port() -> u16 {
    9000
}
fn default_heartbeat_interval() -> i32 {
    300
}
fn default_shutdown_timeout() -> u64 {
    30
}
fn default_db_type() -> DbType {
    DbType::Sqlite
}
fn default_sqlite_path() -> String {
    "./ocpp.db".into()
}
fn default_pg_host() -> String {
    "localhost".into()
}
fn default_pg_port() -> u16 {
    5432
}
fn default_pg_user() -> String {
    "ocpp".into()
}
fn default_pg_database() -> String {
    "ocpp".into()
}
fn default_jwt_secret() -> String {
    "super-secret-key-change-in-production".into()
}
fn default_jwt_expiration() -> i64 {
    24
}
fn default_admin_username() -> String {
    "admin".into()
}
fn default_admin_email() -> String {
    "admin@texnouz.com".into()
}
fn default_admin_password() -> String {
    "admin123".into()
}
fn default_log_level() -> String {
    "info".into()
}

// ── Trait implementations ──────────────────────────────────────

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseSettings::default(),
            security: SecurityConfig::default(),
            admin: AdminConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            api_host: default_host(),
            api_port: default_api_port(),
            ws_host: default_host(),
            ws_port: default_ws_port(),
            heartbeat_interval: default_heartbeat_interval(),
            shutdown_timeout: default_shutdown_timeout(),
        }
    }
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            driver: default_db_type(),
            sqlite: SqliteConfig::default(),
            postgres: PostgresConfig::default(),
        }
    }
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: default_sqlite_path(),
        }
    }
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            host: default_pg_host(),
            port: default_pg_port(),
            username: default_pg_user(),
            password: String::new(),
            database: default_pg_database(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: default_jwt_secret(),
            jwt_expiration_hours: default_jwt_expiration(),
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            username: default_admin_username(),
            email: default_admin_email(),
            password: default_admin_password(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

// ── Convenience converters ─────────────────────────────────────

impl DatabaseSettings {
    /// Build the SeaORM-compatible connection URL
    pub fn connection_url(&self) -> String {
        match self.driver {
            DbType::Sqlite => {
                format!("sqlite://{}?mode=rwc", self.sqlite.path)
            }
            DbType::Postgres => {
                format!(
                    "postgres://{}:{}@{}:{}/{}",
                    self.postgres.username,
                    self.postgres.password,
                    self.postgres.host,
                    self.postgres.port,
                    self.postgres.database,
                )
            }
        }
    }
}

/// Old `Config` compatibility wrapper used by OcppServer
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub heartbeat_interval: i32,
}

impl Config {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            heartbeat_interval: 300,
        }
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 9000,
            heartbeat_interval: 300,
        }
    }
}

impl From<&AppConfig> for Config {
    fn from(app: &AppConfig) -> Self {
        Self {
            host: app.server.ws_host.clone(),
            port: app.server.ws_port,
            heartbeat_interval: app.server.heartbeat_interval,
        }
    }
}

// ── File I/O ───────────────────────────────────────────────────

/// Default configuration directory and file
pub fn default_config_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("texnouz-ocpp")
        .join("config.toml")
}

impl AppConfig {
    /// Load configuration from a TOML file.
    /// If the file doesn't exist, creates one with defaults.
    pub fn load(path: &Path) -> Result<Self, String> {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
            let cfg: AppConfig = toml::from_str(&content)
                .map_err(|e| format!("Invalid TOML in {}: {}", path.display(), e))?;
            cfg.validate()?;
            Ok(cfg)
        } else {
            let cfg = AppConfig::default();
            cfg.save(path)?;
            Ok(cfg)
        }
    }

    /// Validate the configuration for common mistakes.
    pub fn validate(&self) -> Result<(), String> {
        let mut errors = Vec::new();

        // Server ports must be valid and distinct
        if self.server.api_port == self.server.ws_port
            && self.server.api_host == self.server.ws_host
        {
            errors.push(format!(
                "API port ({}) and WebSocket port ({}) must be different when bound to the same host",
                self.server.api_port, self.server.ws_port
            ));
        }

        if self.server.heartbeat_interval < 10 {
            errors.push(format!(
                "Heartbeat interval ({}) must be at least 10 seconds",
                self.server.heartbeat_interval
            ));
        }

        if self.server.shutdown_timeout < 5 {
            errors.push(format!(
                "Shutdown timeout ({}) must be at least 5 seconds",
                self.server.shutdown_timeout
            ));
        }

        // Security: JWT secret must not be the default in non-dev environments
        if self.security.jwt_secret == default_jwt_secret() {
            // Just a warning — we log it but don't block startup
            eprintln!(
                "⚠️  WARNING: Using default JWT secret. Set [security].jwt_secret in config for production!"
            );
        }

        if self.security.jwt_secret.len() < 16 {
            errors.push(format!(
                "JWT secret must be at least 16 characters (got {})",
                self.security.jwt_secret.len()
            ));
        }

        if self.security.jwt_expiration_hours < 1 || self.security.jwt_expiration_hours > 720 {
            errors.push(format!(
                "JWT expiration ({} hours) must be between 1 and 720",
                self.security.jwt_expiration_hours
            ));
        }

        // Database
        if self.database.driver == DbType::Postgres && self.database.postgres.password.is_empty() {
            errors.push("PostgreSQL password must not be empty".to_string());
        }

        // Admin
        if self.admin.password.len() < 6 {
            errors.push(format!(
                "Admin password must be at least 6 characters (got {})",
                self.admin.password.len()
            ));
        }

        // Logging level
        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_levels.contains(&self.logging.level.to_lowercase().as_str()) {
            errors.push(format!(
                "Invalid log level '{}'. Valid: {:?}",
                self.logging.level, valid_levels
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Configuration validation failed:\n  • {}",
                errors.join("\n  • ")
            ))
        }
    }

    /// Persist current configuration to a TOML file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create dirs {}: {}", parent.display(), e))?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| format!("Serialization error: {}", e))?;

        let header = "# Texnouz OCPP Central System — Configuration\n\
                      # Изменения вступят в силу после перезапуска сервера.\n\n";

        std::fs::write(path, format!("{}{}", header, content))
            .map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
        Ok(())
    }
}
