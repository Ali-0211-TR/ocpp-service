//!
//! OCPP 1.6 WebSocket server for managing EV charging stations.
//! Reads configuration from TOML file (~/.config/texnouz-ocpp/config.toml).

use std::sync::Arc;

use sea_orm_migration::MigratorTrait;
use tracing::{error, info, warn};

use metrics_exporter_prometheus;
use texnouz_ocpp::application::commands::{create_command_dispatcher, create_command_sender};
use texnouz_ocpp::application::services::{BillingService, ChargePointService, HeartbeatMonitor};
use texnouz_ocpp::application::session::SessionRegistry;
use texnouz_ocpp::config::AppConfig;
use texnouz_ocpp::domain::OcppVersion;
use texnouz_ocpp::infrastructure::crypto::jwt::JwtConfig;
use texnouz_ocpp::infrastructure::database::migrator::Migrator;
use texnouz_ocpp::interfaces::ws::{
    OcppServer, ProtocolAdapters, V16AdapterFactory, V201AdapterFactory,
};
use texnouz_ocpp::shared::shutdown::ShutdownCoordinator;
use texnouz_ocpp::{
    create_api_router, create_event_bus, default_config_path, init_database, Config,
    DatabaseConfig, SeaOrmRepositoryProvider,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Load configuration ─────────────────────────────────────
    let config_path = std::env::var("OCPP_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| default_config_path());
    let app_cfg = match AppConfig::load(&config_path) {
        Ok(cfg) => {
            // Initialize logging with configured level
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&cfg.logging.level)),
                )
                .init();
            info!("Configuration loaded from {}", config_path.display());
            cfg
        }
        Err(e) => {
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
                .init();
            error!("Failed to load config: {}. Using defaults.", e);
            AppConfig::default()
        }
    };

    info!("Starting Texnouz OCPP Central System...");

    // ── Prometheus metrics recorder (must be installed before any metrics calls) ──
    let prometheus_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .expect("Failed to install Prometheus metrics recorder");
    info!("📊 Prometheus metrics recorder installed");

    // ── Build sub-configs from AppConfig ───────────────────────
    let config = Config::from(&app_cfg);
    let db_config = DatabaseConfig {
        url: app_cfg.database.connection_url(),
    };
    info!("Database: {}", db_config.url);

    let jwt_config = JwtConfig {
        secret: app_cfg.security.jwt_secret.clone(),
        expiration_hours: app_cfg.security.jwt_expiration_hours,
        issuer: "texnouz-ocpp".to_string(),
    };
    info!(
        "JWT configured with {}h token expiration",
        jwt_config.expiration_hours
    );

    // ── Database ───────────────────────────────────────────────
    let db = match init_database(&db_config).await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            return Err(e.into());
        }
    };

    info!("Running database migrations...");
    if let Err(e) = Migrator::up(&db, None).await {
        error!("Failed to run migrations: {}", e);
        return Err(e.into());
    }
    info!("Migrations completed");

    // Create default admin user if not exists
    create_default_admin(&db, &app_cfg).await;

    // Initialize repository provider (replaces monolithic Storage)
    let repos: Arc<dyn texnouz_ocpp::domain::RepositoryProvider> =
        Arc::new(SeaOrmRepositoryProvider::new(db.clone()));

    // Initialize services
    let service = Arc::new(ChargePointService::new(repos.clone()));
    let billing_service = Arc::new(BillingService::new(repos.clone()));

    // Initialize event bus for real-time notifications
    let event_bus = create_event_bus();
    info!("🔔 Event bus initialized for real-time notifications");

    // ── Session & Command infrastructure (shared across WS + API) ──
    let session_registry = SessionRegistry::shared();
    let command_sender = create_command_sender(session_registry.clone());
    let command_dispatcher = create_command_dispatcher(command_sender.clone(), session_registry.clone());

    // ── Protocol adapters (one per supported OCPP version) ─────
    let v16_factory = Arc::new(V16AdapterFactory::new(
        service.clone(),
        billing_service.clone(),
        command_sender.clone(),
        event_bus.clone(),
    ));

    let mut protocol_adapters = ProtocolAdapters::new();
    protocol_adapters.register(OcppVersion::V16, v16_factory);

    // ── OCPP 2.0.1 adapter ────────────────────────────────────
    let v201_factory = Arc::new(V201AdapterFactory::new(
        service.clone(),
        billing_service.clone(),
        command_sender.clone(),
        event_bus.clone(),
    ));
    protocol_adapters.register(OcppVersion::V201, v201_factory);
    // Future: protocol_adapters.register(OcppVersion::V21,  v21_factory);
    let protocol_adapters = Arc::new(protocol_adapters);

    // Initialize shutdown coordinator
    let shutdown = ShutdownCoordinator::new(app_cfg.server.shutdown_timeout);
    let shutdown_signal = shutdown.signal();

    // Start listening for shutdown signals (SIGTERM, SIGINT)
    shutdown.start_signal_listener();

    // Create unified OCPP WebSocket server with shutdown support
    let server = OcppServer::new(
        config.clone(),
        protocol_adapters,
        session_registry.clone(),
        command_sender.clone(),
        event_bus.clone(),
        repos.clone(),
        app_cfg.rate_limit.ws_connections_per_minute,
    )
    .with_shutdown(shutdown_signal.clone());

    // Start Heartbeat Monitor
    let heartbeat_monitor = Arc::new(HeartbeatMonitor::new(
        repos.clone(),
        session_registry.clone(),
    ));
    heartbeat_monitor.start(shutdown_signal.clone());

    // Create REST API router
    let api_router = create_api_router(
        repos,
        session_registry,
        command_dispatcher,
        db.clone(),
        jwt_config,
        heartbeat_monitor,
        event_bus,
        service,
        billing_service,
        &app_cfg,
        prometheus_handle,
    );

    // Start REST API server with graceful shutdown
    let api_addr = format!("{}:{}", app_cfg.server.api_host, app_cfg.server.api_port);
    let listener = tokio::net::TcpListener::bind(&api_addr).await?;
    info!("REST API server listening on http://{}", api_addr);
    info!("Swagger UI available at http://{}/docs/", api_addr);

    let api_shutdown = shutdown_signal.clone();
    let api_server = axum::serve(
        listener,
        api_router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        api_shutdown.wait().await;
        info!("🛑 REST API server received shutdown signal");
    });

    // Run both servers concurrently
    info!("🚀 All servers started. Press Ctrl+C to shutdown gracefully.");

    let ws_result = tokio::spawn(async move { server.run().await });

    let api_result = tokio::spawn(async move { api_server.await });

    // Wait for shutdown signal or server error
    tokio::select! {
        result = ws_result => {
            match result {
                Ok(Ok(())) => info!("WebSocket server stopped"),
                Ok(Err(e)) => error!("WebSocket server error: {}", e),
                Err(e) => error!("WebSocket server task panicked: {}", e),
            }
        }
        result = api_result => {
            match result {
                Ok(Ok(())) => info!("REST API server stopped"),
                Ok(Err(e)) => error!("REST API server error: {}", e),
                Err(e) => error!("REST API server task panicked: {}", e),
            }
        }
    }

    // Perform final cleanup
    info!("🧹 Performing final cleanup...");

    if let Err(e) = db.close().await {
        warn!("Error closing database connection: {}", e);
    } else {
        info!("✅ Database connection closed");
    }

    info!("👋 Texnouz OCPP Central System shutdown complete");
    Ok(())
}

/// Create default admin user if no users exist
async fn create_default_admin(db: &sea_orm::DatabaseConnection, app_cfg: &AppConfig) {
    use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, Set};
    use texnouz_ocpp::infrastructure::crypto::password::hash_password;
    use texnouz_ocpp::infrastructure::database::entities::user::{self, UserRole};

    let users_count = user::Entity::find().count(db).await.unwrap_or(0);

    if users_count == 0 {
        info!("Creating default admin user...");

        let admin_email = app_cfg.admin.email.clone();
        let admin_username = app_cfg.admin.username.clone();
        let admin_password = app_cfg.admin.password.clone();

        let password_hash = match hash_password(&admin_password) {
            Ok(hash) => hash,
            Err(e) => {
                error!("Failed to hash admin password: {}", e);
                return;
            }
        };

        let admin = user::ActiveModel {
            id: Set(uuid::Uuid::new_v4().to_string()),
            username: Set(admin_username),
            email: Set(admin_email.clone()),
            password_hash: Set(password_hash),
            role: Set(UserRole::Admin),
            is_active: Set(true),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            last_login_at: Set(None),
        };

        match admin.insert(db).await {
            Ok(_) => {
                info!("Default admin created: {}", admin_email);
                info!("⚠️  Please change the admin password immediately!");
            }
            Err(e) => {
                error!("Failed to create admin user: {}", e);
            }
        }
    }
}
