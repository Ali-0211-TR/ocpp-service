//! Texnouz OCPP Central System
//!
//! OCPP 1.6 WebSocket server for managing EV charging stations.
//! Reads configuration from TOML file (~/.config/texnouz-ocpp/config.toml).

use std::sync::Arc;

use log::{error, info, warn};
use sea_orm_migration::MigratorTrait;

use texnouz_ocpp::application::services::{BillingService, ChargePointService, HeartbeatMonitor};
use texnouz_ocpp::auth::jwt::JwtConfig;
use texnouz_ocpp::config::AppConfig;
use texnouz_ocpp::infrastructure::database::migrator::Migrator;
use texnouz_ocpp::infrastructure::server::ShutdownCoordinator;
use texnouz_ocpp::infrastructure::{DatabaseStorage, OcppServer};
use texnouz_ocpp::notifications::create_event_bus;
use texnouz_ocpp::{create_api_router, default_config_path, init_database, Config, DatabaseConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Load configuration ─────────────────────────────────────
    // Allow overriding config path via OCPP_CONFIG env var (used by desktop app)
    let config_path = std::env::var("OCPP_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| default_config_path());
    let app_cfg = match AppConfig::load(&config_path) {
        Ok(cfg) => {
            // Initialize logging with configured level
            env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or(&cfg.logging.level),
            )
            .init();
            info!("Configuration loaded from {}", config_path.display());
            cfg
        }
        Err(e) => {
            env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or("info"),
            )
            .init();
            error!("Failed to load config: {}. Using defaults.", e);
            AppConfig::default()
        }
    };

    info!("Starting Texnouz OCPP Central System...");

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

    // Initialize storage (using database)
    let storage: Arc<dyn texnouz_ocpp::infrastructure::Storage> =
        Arc::new(DatabaseStorage::new(db.clone()));

    // Initialize services
    let service = Arc::new(ChargePointService::new(storage.clone()));
    let billing_service = Arc::new(BillingService::new(storage.clone()));

    // Initialize event bus for real-time notifications
    let event_bus = create_event_bus();
    info!("🔔 Event bus initialized for real-time notifications");

    // Initialize shutdown coordinator
    let shutdown = ShutdownCoordinator::new(app_cfg.server.shutdown_timeout);
    let shutdown_signal = shutdown.signal();

    // Start listening for shutdown signals (SIGTERM, SIGINT)
    shutdown.start_signal_listener();

    // Create OCPP WebSocket server with shutdown support
    let server = OcppServer::new(config.clone(), service.clone(), billing_service, event_bus.clone())
        .with_shutdown(shutdown_signal.clone());

    // Get session manager and command sender for API
    let session_manager = server.get_session_manager();
    let command_sender = server.command_sender().clone();

    // Start Heartbeat Monitor
    let heartbeat_monitor = Arc::new(HeartbeatMonitor::new(
        storage.clone(),
        session_manager.clone(),
    ));
    heartbeat_monitor.start(shutdown_signal.clone());

    // Create REST API router (pass heartbeat_monitor for status endpoints)
    let api_router = create_api_router(
        storage,
        session_manager,
        command_sender,
        db.clone(),
        jwt_config,
        heartbeat_monitor,
        event_bus,
        service,
    );

    // Start REST API server with graceful shutdown
    let api_addr = format!("{}:{}", app_cfg.server.api_host, app_cfg.server.api_port);
    let listener = tokio::net::TcpListener::bind(&api_addr).await?;
    info!("REST API server listening on http://{}", api_addr);
    info!("Swagger UI available at http://{}/swagger-ui/", api_addr);

    // Create REST API server with graceful shutdown
    let api_shutdown = shutdown_signal.clone();
    let api_server = axum::serve(listener, api_router)
        .with_graceful_shutdown(async move {
            api_shutdown.wait().await;
            info!("🛑 REST API server received shutdown signal");
        });

    // Run both servers concurrently
    info!("🚀 All servers started. Press Ctrl+C to shutdown gracefully.");
    
    let ws_result = tokio::spawn(async move {
        server.run().await
    });

    let api_result = tokio::spawn(async move {
        api_server.await
    });

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
    
    // Close database connection
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
    use texnouz_ocpp::auth::password::hash_password;
    use texnouz_ocpp::infrastructure::database::entities::user::{self, UserRole};

    // Check if any users exist
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
