//! Reusable OCPP Central System server runtime.
//!
//! Provides [`ServerHandle`] that encapsulates the full server lifecycle:
//! database init, migrations, OCPP WebSocket server, REST API, heartbeat
//! monitor, metrics, and graceful shutdown.
//!
//! Both the CLI binary and the Tauri desktop app use this to start/stop
//! the CSMS without duplicating 300+ lines of bootstrap code.

use std::sync::Arc;

use sea_orm::DatabaseConnection;
use sea_orm_migration::MigratorTrait;
use tracing::{error, info, warn};

use crate::application::charging::commands::dispatcher::{
    create_command_dispatcher, SharedCommandDispatcher,
};
use crate::application::charging::commands::{create_command_sender, SharedCommandSender};
use crate::application::charging::services::device_report::DeviceReportStore;
use crate::application::services::{BillingService, ChargePointService, HeartbeatMonitor};
use crate::application::session::SessionRegistry;
use crate::config::AppConfig;
use crate::domain::{OcppVersion, RepositoryProvider};
use crate::infrastructure::crypto::jwt::JwtConfig;
use crate::infrastructure::database::migrator::Migrator;
use crate::interfaces::ws::{OcppServer, ProtocolAdapters, V16AdapterFactory, V201AdapterFactory};
use crate::shared::shutdown::{ShutdownCoordinator, ShutdownSignal};
use crate::{
    create_api_router, create_event_bus, init_database, Config, DatabaseConfig,
    SeaOrmRepositoryProvider, SharedEventBus,
};

// ── Options ────────────────────────────────────────────────────────

/// Options for starting the OCPP Central System.
pub struct ServerOptions {
    /// Application configuration.
    pub config: AppConfig,
    /// Run database migrations on startup (default: true).
    pub auto_migrate: bool,
    /// Create default admin user if none exists (default: true).
    pub create_default_admin: bool,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            auto_migrate: true,
            create_default_admin: true,
        }
    }
}

// ── ServerHandle ───────────────────────────────────────────────────

/// Handle to a running OCPP Central System.
///
/// Provides access to internal server components (repos, session registry,
/// event bus) and methods for graceful shutdown.
///
/// # Examples
///
/// ```rust,no_run
/// use texnouz_csms::server::{ServerHandle, ServerOptions};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let handle = ServerHandle::start(ServerOptions::default()).await?;
///     // ... wait for shutdown signal ...
///     handle.shutdown().await;
///     Ok(())
/// }
/// ```
pub struct ServerHandle {
    /// Shared event bus for real-time notifications.
    pub event_bus: SharedEventBus,
    /// Repository provider for data access.
    pub repos: Arc<dyn RepositoryProvider>,
    /// Active WebSocket session registry.
    pub session_registry: Arc<SessionRegistry>,
    /// Command dispatcher for sending OCPP commands.
    pub command_dispatcher: SharedCommandDispatcher,
    /// Command sender for low-level message dispatch.
    pub command_sender: SharedCommandSender,
    /// The configuration the server was started with.
    pub config: AppConfig,
    /// API port the server is listening on.
    pub api_port: u16,
    /// WebSocket port the server is listening on.
    pub ws_port: u16,

    db: DatabaseConnection,
    shutdown: ShutdownCoordinator,
    ws_task: tokio::task::JoinHandle<()>,
    api_task: tokio::task::JoinHandle<()>,
}

impl ServerHandle {
    /// Start the OCPP Central System with the given options.
    ///
    /// This will:
    /// 1. Install Prometheus metrics recorder
    /// 2. Connect to database and run migrations
    /// 3. Create default admin user (if enabled)
    /// 4. Start OCPP WebSocket server (all supported protocol versions)
    /// 5. Start REST API server (with Swagger UI)
    /// 6. Start heartbeat monitor and reservation expiry tasks
    pub async fn start(opts: ServerOptions) -> Result<Self, Box<dyn std::error::Error>> {
        let app_cfg = opts.config;

        info!("Starting Texnouz CSMS...");

        // ── Prometheus metrics recorder ────────────────────────
        // The global metrics recorder can only be installed once per process.
        // On restart (stop + start within the same process) we must reuse it.
        use std::sync::OnceLock;
        static PROM_HANDLE: OnceLock<metrics_exporter_prometheus::PrometheusHandle> =
            OnceLock::new();

        let prometheus_handle = PROM_HANDLE
            .get_or_init(|| {
                let h = metrics_exporter_prometheus::PrometheusBuilder::new()
                    .install_recorder()
                    .expect("Failed to install Prometheus metrics recorder");
                info!("📊 Prometheus metrics recorder installed");
                h
            })
            .clone();
        info!("📊 Prometheus metrics recorder ready");

        // ── Build sub-configs ──────────────────────────────────
        let config = Config::from(&app_cfg);
        let db_config = DatabaseConfig {
            url: app_cfg.database.connection_url(),
            pool: app_cfg.database.pool.clone(),
        };
        info!("Database: {}", db_config.url);

        let jwt_config = JwtConfig {
            secret: app_cfg.security.jwt_secret.clone(),
            expiration_hours: app_cfg.security.jwt_expiration_hours,
            issuer: "texnouz-csms".to_string(),
        };
        info!(
            "JWT configured with {}h token expiration",
            jwt_config.expiration_hours
        );

        // ── Database ───────────────────────────────────────────
        let db = init_database(&db_config).await?;

        if opts.auto_migrate {
            info!("Running database migrations...");
            Migrator::up(&db, None).await?;
            info!("Migrations completed");
        }

        if opts.create_default_admin {
            create_default_admin(&db, &app_cfg).await;
        }

        // ── Repositories & Services ────────────────────────────
        let repos: Arc<dyn RepositoryProvider> =
            Arc::new(SeaOrmRepositoryProvider::new(db.clone()));
        let service = Arc::new(ChargePointService::new(repos.clone()));
        let billing_service = Arc::new(BillingService::new(repos.clone()));

        // ── Event Bus ──────────────────────────────────────────
        let event_bus = create_event_bus();
        info!("🔔 Event bus initialized for real-time notifications");

        // ── Session & Command infrastructure ───────────────────
        let session_registry = SessionRegistry::shared();
        let command_sender = create_command_sender(session_registry.clone());
        let command_dispatcher =
            create_command_dispatcher(command_sender.clone(), session_registry.clone());

        // ── Protocol adapters ──────────────────────────────────
        let v16_factory = Arc::new(V16AdapterFactory::new(
            service.clone(),
            billing_service.clone(),
            command_sender.clone(),
            event_bus.clone(),
        ));

        let device_report_store = Arc::new(DeviceReportStore::new());

        let v201_factory = Arc::new(V201AdapterFactory::new(
            service.clone(),
            billing_service.clone(),
            command_sender.clone(),
            event_bus.clone(),
            device_report_store.clone(),
        ));

        let mut protocol_adapters = ProtocolAdapters::new();
        protocol_adapters.register(OcppVersion::V16, v16_factory);
        protocol_adapters.register(OcppVersion::V201, v201_factory);
        let protocol_adapters = Arc::new(protocol_adapters);

        // ── Shutdown coordinator ───────────────────────────────
        let shutdown = ShutdownCoordinator::new(app_cfg.server.shutdown_timeout);
        let shutdown_signal = shutdown.signal();

        // ── OCPP WebSocket server ──────────────────────────────
        let server = OcppServer::new(
            config.clone(),
            protocol_adapters,
            session_registry.clone(),
            command_sender.clone(),
            event_bus.clone(),
            repos.clone(),
            app_cfg.rate_limit.ws_connections_per_minute,
            app_cfg.ws_auth.clone(),
        )
        .with_shutdown(shutdown_signal.clone());

        // ── Background tasks ───────────────────────────────────
        let heartbeat_monitor = Arc::new(HeartbeatMonitor::new(
            repos.clone(),
            session_registry.clone(),
        ));
        heartbeat_monitor.start(shutdown_signal.clone());

        crate::application::charging::services::start_reservation_expiry_task(
            repos.clone(),
            shutdown_signal.clone(),
            60, // check every 60 seconds
        );

        // ── REST API server ────────────────────────────────────
        let api_router = create_api_router(
            repos.clone(),
            session_registry.clone(),
            command_dispatcher.clone(),
            db.clone(),
            jwt_config,
            heartbeat_monitor,
            event_bus.clone(),
            service,
            billing_service,
            &app_cfg,
            prometheus_handle,
            device_report_store,
        );

        let api_port = app_cfg.server.api_port;
        let ws_port = app_cfg.server.ws_port;
        let api_addr = format!("{}:{}", app_cfg.server.api_host, api_port);
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

        info!("🚀 All servers started.");

        // ── Spawn server tasks ─────────────────────────────────
        let ws_task = tokio::spawn(async move {
            if let Err(e) = server.run().await {
                error!("WebSocket server error: {}", e);
            }
        });

        let api_task = tokio::spawn(async move {
            if let Err(e) = api_server.await {
                error!("REST API server error: {}", e);
            }
        });

        Ok(Self {
            event_bus,
            repos,
            session_registry,
            command_dispatcher,
            command_sender,
            config: app_cfg,
            api_port,
            ws_port,
            db,
            shutdown,
            ws_task,
            api_task,
        })
    }

    /// Get a cloneable shutdown signal.
    pub fn shutdown_signal(&self) -> ShutdownSignal {
        self.shutdown.signal()
    }

    /// Install OS signal listeners (SIGTERM, SIGINT) that trigger shutdown.
    ///
    /// Typically used by the CLI binary. Desktop apps manage their own lifecycle.
    pub fn install_signal_handler(&self) {
        self.shutdown.start_signal_listener();
    }

    /// Trigger graceful shutdown (non-blocking).
    ///
    /// Sends the shutdown signal to all server components. Call [`wait`] to
    /// block until everything has stopped.
    pub fn trigger_shutdown(&self) {
        self.shutdown.signal().trigger();
    }

    /// Wait for the server to fully stop after shutdown has been triggered.
    pub async fn wait(self) {
        info!("⏳ Waiting for server tasks to complete...");

        // Wait for both server tasks
        tokio::select! {
            result = self.ws_task => {
                match result {
                    Ok(()) => info!("WebSocket server stopped"),
                    Err(e) => error!("WebSocket server task panicked: {}", e),
                }
            }
            result = self.api_task => {
                match result {
                    Ok(()) => info!("REST API server stopped"),
                    Err(e) => error!("REST API server task panicked: {}", e),
                }
            }
        }

        // Close database connection
        if let Err(e) = self.db.close().await {
            warn!("Error closing database connection: {}", e);
        } else {
            info!("✅ Database connection closed");
        }

        info!("👋 Texnouz CSMS shutdown complete");
    }

    /// Trigger shutdown and wait for completion.
    pub async fn shutdown(self) {
        info!("🛑 Shutting down OCPP Central System...");
        self.trigger_shutdown();
        self.wait().await;
    }

    /// Check if the server is still running.
    pub fn is_running(&self) -> bool {
        !self.ws_task.is_finished() || !self.api_task.is_finished()
    }
}

// ── Helpers ────────────────────────────────────────────────────────

/// Create default admin user if no users exist in the database.
async fn create_default_admin(db: &DatabaseConnection, app_cfg: &AppConfig) {
    use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, Set};

    use crate::infrastructure::crypto::password::hash_password;
    use crate::infrastructure::database::entities::user::{self, UserRole};

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

/// Initialize tracing (logging) from the application config.
///
/// Call this once at process startup (before [`ServerHandle::start`]).
pub fn init_tracing(config: &AppConfig) {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.logging.level));

    match config.logging.format.to_lowercase().as_str() {
        "json" => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        _ => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer())
                .init();
        }
    }
}
