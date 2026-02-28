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

    /// CORS settings
    #[serde(default)]
    pub cors: CorsConfig,

    /// Rate limiting
    #[serde(default)]
    pub rate_limit: RateLimitConfig,

    /// WebSocket authentication for charge points
    #[serde(default)]
    pub ws_auth: WsAuthConfig,
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

    /// Connection pool settings
    #[serde(default)]
    pub pool: DatabasePoolConfig,
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

    /// Log output format: "text" (human-readable) or "json" (structured)
    #[serde(default = "default_log_format")]
    pub format: String,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// List of allowed origins. Empty list or ["*"] means allow any origin.
    /// Examples: ["https://your-frontend.com", "http://localhost:3000"]
    #[serde(default = "default_cors_origins")]
    pub allowed_origins: Vec<String>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum API requests per minute per IP (general endpoints)
    #[serde(default = "default_api_rpm")]
    pub api_requests_per_minute: u32,

    /// Maximum login attempts per minute per IP
    #[serde(default = "default_login_rpm")]
    pub login_attempts_per_minute: u32,

    /// Maximum new WebSocket connections per minute per IP
    #[serde(default = "default_ws_rpm")]
    pub ws_connections_per_minute: u32,
}

/// Database connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabasePoolConfig {
    /// Maximum number of connections in the pool
    #[serde(default = "default_pool_max_connections")]
    pub max_connections: u32,

    /// Minimum number of idle connections to keep in the pool
    #[serde(default = "default_pool_min_connections")]
    pub min_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_pool_connect_timeout")]
    pub connect_timeout_seconds: u64,

    /// Idle connection timeout in seconds (0 = no timeout)
    #[serde(default = "default_pool_idle_timeout")]
    pub idle_timeout_seconds: u64,

    /// Maximum connection lifetime in seconds (0 = no limit)
    #[serde(default = "default_pool_max_lifetime")]
    pub max_lifetime_seconds: u64,
}

/// WebSocket authentication configuration for charge point connections.
///
/// Controls how OCPP charge points authenticate during the WebSocket handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsAuthConfig {
    /// Authentication mode:
    /// - `"none"` — no WS authentication (default, dev mode)
    /// - `"basic"` — HTTP Basic Auth (OCPP Security Profile 1)
    ///   Charge point sends `Authorization: Basic base64(charge_point_id:password)`
    #[serde(default = "default_ws_auth_mode")]
    pub mode: String,

    /// Reject WebSocket connections from charge points not registered in the database.
    /// When `false`, unknown charge points can connect and self-register via BootNotification.
    #[serde(default = "default_reject_unknown")]
    pub reject_unknown_charge_points: bool,
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
fn default_log_format() -> String {
    "text".into()
}
fn default_cors_origins() -> Vec<String> {
    vec!["*".into()]
}
fn default_api_rpm() -> u32 {
    100
}
fn default_login_rpm() -> u32 {
    10
}
fn default_ws_rpm() -> u32 {
    600
}
fn default_pool_max_connections() -> u32 {
    10
}
fn default_pool_min_connections() -> u32 {
    2
}
fn default_pool_connect_timeout() -> u64 {
    5
}
fn default_pool_idle_timeout() -> u64 {
    300
}
fn default_pool_max_lifetime() -> u64 {
    1800
}
fn default_ws_auth_mode() -> String {
    "none".into()
}
fn default_reject_unknown() -> bool {
    false
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
            cors: CorsConfig::default(),
            rate_limit: RateLimitConfig::default(),
            ws_auth: WsAuthConfig::default(),
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
            pool: DatabasePoolConfig::default(),
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
            format: default_log_format(),
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: default_cors_origins(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            api_requests_per_minute: default_api_rpm(),
            login_attempts_per_minute: default_login_rpm(),
            ws_connections_per_minute: default_ws_rpm(),
        }
    }
}

impl Default for DatabasePoolConfig {
    fn default() -> Self {
        Self {
            max_connections: default_pool_max_connections(),
            min_connections: default_pool_min_connections(),
            connect_timeout_seconds: default_pool_connect_timeout(),
            idle_timeout_seconds: default_pool_idle_timeout(),
            max_lifetime_seconds: default_pool_max_lifetime(),
        }
    }
}

impl Default for WsAuthConfig {
    fn default() -> Self {
        Self {
            mode: default_ws_auth_mode(),
            reject_unknown_charge_points: default_reject_unknown(),
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
        .join("texnouz-csms")
        .join("config.toml")
}

impl AppConfig {
    /// Load configuration from a TOML file.
    /// If the file doesn't exist, creates one with defaults.
    /// Environment variables override TOML values (highest priority).
    pub fn load(path: &Path) -> Result<Self, String> {
        let mut cfg = if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
            toml::from_str(&content)
                .map_err(|e| format!("Invalid TOML in {}: {}", path.display(), e))?
        } else {
            let cfg = AppConfig::default();
            cfg.save(path)?;
            cfg
        };

        // ── Environment variable overrides ─────────────────────
        // Env vars have highest priority, overriding TOML values.
        cfg.apply_env_overrides();

        cfg.validate()?;
        Ok(cfg)
    }

    /// Apply environment variable overrides for sensitive config values.
    ///
    /// Supported variables:
    /// - `OCPP_JWT_SECRET` → `[security].jwt_secret`
    /// - `OCPP_DB_PASSWORD` → `[database.postgres].password`
    /// - `OCPP_ADMIN_PASSWORD` → `[admin].password`
    /// - `OCPP_ADMIN_USERNAME` → `[admin].username`
    /// - `OCPP_ADMIN_EMAIL` → `[admin].email`
    /// - `OCPP_DB_URL` → overrides the entire database connection URL (driver-dependent)
    /// - `OCPP_LOG_LEVEL` → `[logging].level`
    /// - `OCPP_LOG_FORMAT` → `[logging].format`
    /// - `OCPP_API_PORT` → `[server].api_port`
    /// - `OCPP_WS_PORT` → `[server].ws_port`
    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("OCPP_JWT_SECRET") {
            self.security.jwt_secret = v;
        }
        if let Ok(v) = std::env::var("OCPP_DB_PASSWORD") {
            self.database.postgres.password = v;
        }
        if let Ok(v) = std::env::var("OCPP_ADMIN_PASSWORD") {
            self.admin.password = v;
        }
        if let Ok(v) = std::env::var("OCPP_ADMIN_USERNAME") {
            self.admin.username = v;
        }
        if let Ok(v) = std::env::var("OCPP_ADMIN_EMAIL") {
            self.admin.email = v;
        }
        if let Ok(v) = std::env::var("OCPP_LOG_LEVEL") {
            self.logging.level = v;
        }
        if let Ok(v) = std::env::var("OCPP_LOG_FORMAT") {
            self.logging.format = v;
        }
        if let Ok(v) = std::env::var("OCPP_API_PORT") {
            if let Ok(port) = v.parse::<u16>() {
                self.server.api_port = port;
            }
        }
        if let Ok(v) = std::env::var("OCPP_WS_PORT") {
            if let Ok(port) = v.parse::<u16>() {
                self.server.ws_port = port;
            }
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

        let valid_formats = ["text", "json"];
        if !valid_formats.contains(&self.logging.format.to_lowercase().as_str()) {
            errors.push(format!(
                "Invalid log format '{}'. Valid: {:?}",
                self.logging.format, valid_formats
            ));
        }

        // WebSocket auth mode
        let valid_ws_modes = ["none", "basic"];
        if !valid_ws_modes.contains(&self.ws_auth.mode.to_lowercase().as_str()) {
            errors.push(format!(
                "Invalid ws_auth.mode '{}'. Valid: {:?}",
                self.ws_auth.mode, valid_ws_modes
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

        let header = "# Texnouz CSMS — Configuration\n\
                      # Изменения вступят в силу после перезапуска сервера.\n\n";

        std::fs::write(path, format!("{}{}", header, content))
            .map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_validates() {
        let cfg = AppConfig::default();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn same_port_same_host_is_error() {
        let mut cfg = AppConfig::default();
        cfg.server.api_port = 8080;
        cfg.server.ws_port = 8080;
        cfg.server.api_host = "0.0.0.0".into();
        cfg.server.ws_host = "0.0.0.0".into();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("must be different"));
    }

    #[test]
    fn same_port_different_host_is_ok() {
        let mut cfg = AppConfig::default();
        cfg.server.api_port = 8080;
        cfg.server.ws_port = 8080;
        cfg.server.api_host = "127.0.0.1".into();
        cfg.server.ws_host = "0.0.0.0".into();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn short_jwt_secret_is_error() {
        let mut cfg = AppConfig::default();
        cfg.security.jwt_secret = "short".into();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("JWT secret must be at least 16"));
    }

    #[test]
    fn jwt_expiration_out_of_range() {
        let mut cfg = AppConfig::default();
        cfg.security.jwt_expiration_hours = 0;
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("JWT expiration"));
    }

    #[test]
    fn heartbeat_too_low() {
        let mut cfg = AppConfig::default();
        cfg.server.heartbeat_interval = 5;
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("Heartbeat interval"));
    }

    #[test]
    fn shutdown_timeout_too_low() {
        let mut cfg = AppConfig::default();
        cfg.server.shutdown_timeout = 2;
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("Shutdown timeout"));
    }

    #[test]
    fn invalid_log_level() {
        let mut cfg = AppConfig::default();
        cfg.logging.level = "verbose".into();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("Invalid log level"));
    }

    #[test]
    fn invalid_log_format() {
        let mut cfg = AppConfig::default();
        cfg.logging.format = "xml".into();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("Invalid log format"));
    }

    #[test]
    fn json_log_format_validates() {
        let mut cfg = AppConfig::default();
        cfg.logging.format = "json".into();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn postgres_empty_password_is_error() {
        let mut cfg = AppConfig::default();
        cfg.database.driver = DbType::Postgres;
        cfg.database.postgres.password = String::new();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("PostgreSQL password"));
    }

    #[test]
    fn admin_short_password_is_error() {
        let mut cfg = AppConfig::default();
        cfg.admin.password = "ab".into();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("Admin password"));
    }

    #[test]
    fn connection_url_sqlite() {
        let cfg = DatabaseSettings::default();
        assert!(cfg.connection_url().starts_with("sqlite://"));
    }

    #[test]
    fn connection_url_postgres() {
        let mut cfg = DatabaseSettings::default();
        cfg.driver = DbType::Postgres;
        cfg.postgres.host = "db.host".into();
        cfg.postgres.port = 5432;
        cfg.postgres.username = "user".into();
        cfg.postgres.password = "pass".into();
        cfg.postgres.database = "mydb".into();
        assert_eq!(cfg.connection_url(), "postgres://user:pass@db.host:5432/mydb");
    }

    #[test]
    fn env_overrides_jwt_secret() {
        let mut cfg = AppConfig::default();
        std::env::set_var("OCPP_JWT_SECRET", "my-super-secret-key-for-jwt-test");
        cfg.apply_env_overrides();
        std::env::remove_var("OCPP_JWT_SECRET");
        assert_eq!(cfg.security.jwt_secret, "my-super-secret-key-for-jwt-test");
    }

    #[test]
    fn env_overrides_ports() {
        let mut cfg = AppConfig::default();
        std::env::set_var("OCPP_API_PORT", "3333");
        std::env::set_var("OCPP_WS_PORT", "4444");
        cfg.apply_env_overrides();
        std::env::remove_var("OCPP_API_PORT");
        std::env::remove_var("OCPP_WS_PORT");
        assert_eq!(cfg.server.api_port, 3333);
        assert_eq!(cfg.server.ws_port, 4444);
    }

    #[test]
    fn env_overrides_invalid_port_ignored() {
        let mut cfg = AppConfig::default();
        let original = cfg.server.api_port;
        std::env::set_var("OCPP_API_PORT", "not_a_number");
        cfg.apply_env_overrides();
        std::env::remove_var("OCPP_API_PORT");
        assert_eq!(cfg.server.api_port, original);
    }

    #[test]
    fn save_and_reload() {
        let dir = std::env::temp_dir().join("ocpp_test_config");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_config.toml");

        let cfg = AppConfig::default();
        cfg.save(&path).unwrap();
        assert!(path.exists());

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(loaded.server.api_port, cfg.server.api_port);
        assert_eq!(loaded.server.ws_port, cfg.server.ws_port);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_from_app_config() {
        let app = AppConfig::default();
        let ws_cfg = Config::from(&app);
        assert_eq!(ws_cfg.host, app.server.ws_host);
        assert_eq!(ws_cfg.port, app.server.ws_port);
        assert_eq!(ws_cfg.heartbeat_interval, app.server.heartbeat_interval);
    }

    #[test]
    fn config_address() {
        let c = Config::new("127.0.0.1", 9000);
        assert_eq!(c.address(), "127.0.0.1:9000");
    }

    #[test]
    fn multiple_validation_errors() {
        let mut cfg = AppConfig::default();
        cfg.security.jwt_secret = "short".into();
        cfg.server.heartbeat_interval = 1;
        cfg.admin.password = "ab".into();
        let err = cfg.validate().unwrap_err();
        // Should contain multiple bullet points
        assert!(err.contains("•"));
        assert!(err.contains("JWT secret"));
        assert!(err.contains("Heartbeat"));
        assert!(err.contains("Admin password"));
    }
}
