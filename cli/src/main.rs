//! Texnouz CSMS — CLI Server
//!
//! Headless OCPP 1.6 / 2.0.1 Central System suitable for deployment
//! as a systemd service, Docker container, or standalone process.
//!
//! ```sh
//! # Run with default config (~/.config/texnouz-csms/config.toml)
//! csms-service
//!
//! # Custom config path
//! csms-service --config /etc/texnouz-csms/config.toml
//!
//! # Override ports
//! csms-service --api-port 8080 --ws-port 9000
//!
//! # Validate config without starting
//! csms-service --check
//! ```

use std::path::PathBuf;

use clap::Parser;
use tracing::{error, info};

use texnouz_csms::config::AppConfig;
use texnouz_csms::server::{init_tracing, ServerHandle, ServerOptions};

/// Texnouz CSMS — OCPP 1.6 / 2.0.1 server for EV charging stations.
#[derive(Parser, Debug)]
#[command(
    name = "csms-service",
    version,
    about = "OCPP Central System for EV charging station management",
    long_about = "Texnouz CSMS — WebSocket + REST API server \
                  for managing EV charging stations via OCPP 1.6 and 2.0.1 protocols.\n\n\
                  Default config: ~/.config/texnouz-csms/config.toml"
)]
struct Cli {
    /// Path to the configuration file (TOML).
    #[arg(short, long, env = "OCPP_CONFIG")]
    config: Option<PathBuf>,

    /// Override the REST API listen port.
    #[arg(long)]
    api_port: Option<u16>,

    /// Override the WebSocket listen port.
    #[arg(long)]
    ws_port: Option<u16>,

    /// Override the log level (trace, debug, info, warn, error).
    #[arg(short, long)]
    log_level: Option<String>,

    /// Validate the configuration file and exit without starting the server.
    #[arg(long)]
    check: bool,

    /// Skip database migrations on startup.
    #[arg(long)]
    no_migrate: bool,

    /// Skip creating the default admin user.
    #[arg(long)]
    no_admin: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // ── Load configuration ─────────────────────────────────────
    let config_path = cli
        .config
        .unwrap_or_else(|| texnouz_csms::default_config_path());

    let mut config = match AppConfig::load(&config_path) {
        Ok(cfg) => {
            // Init tracing first so subsequent logs are formatted properly
            init_tracing(&cfg);
            info!("Configuration loaded from {}", config_path.display());
            cfg
        }
        Err(e) => {
            // Fallback tracing init
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
                .init();
            error!("Failed to load config from {}: {}", config_path.display(), e);
            error!("Using default configuration.");
            AppConfig::default()
        }
    };

    // ── Apply CLI overrides ────────────────────────────────────
    if let Some(port) = cli.api_port {
        info!("CLI override: api_port = {}", port);
        config.server.api_port = port;
    }
    if let Some(port) = cli.ws_port {
        info!("CLI override: ws_port = {}", port);
        config.server.ws_port = port;
    }
    if let Some(ref level) = cli.log_level {
        info!("CLI override: log_level = {}", level);
        config.logging.level = level.clone();
    }

    // ── Config validation mode ─────────────────────────────────
    if cli.check {
        println!("✅ Configuration is valid");
        println!("   Config file : {}", config_path.display());
        println!("   API address : {}:{}", config.server.api_host, config.server.api_port);
        println!("   WS address  : {}:{}", config.server.ws_host, config.server.ws_port);
        println!("   Database    : {}", config.database.connection_url());
        println!("   Log level   : {}", config.logging.level);
        return Ok(());
    }

    // ── Start server ───────────────────────────────────────────
    let handle = ServerHandle::start(ServerOptions {
        config,
        auto_migrate: !cli.no_migrate,
        create_default_admin: !cli.no_admin,
    })
    .await?;

    // Install OS signal handlers (SIGTERM, SIGINT)
    handle.install_signal_handler();

    info!("🚀 Press Ctrl+C to shutdown gracefully.");

    // Wait for shutdown signal, then clean up
    handle.shutdown_signal().wait().await;
    handle.wait().await;

    Ok(())
}
