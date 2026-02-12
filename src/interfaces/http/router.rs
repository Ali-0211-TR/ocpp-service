//! API Router with Swagger UI

use std::sync::Arc;

use axum::{
    extract::FromRef,
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use sea_orm::DatabaseConnection;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::interfaces::http::*;
use crate::application::events::SharedEventBus;
use crate::application::CommandSender;
use crate::application::SharedSessionRegistry;
use crate::application::{ChargePointService, HeartbeatMonitor};
use crate::domain::Storage;
use crate::infrastructure::crypto::jwt::JwtConfig;
use crate::interfaces::http::middleware::{auth_middleware, AuthState};
use crate::interfaces::ws::{create_notification_state, ws_notifications_handler};

use super::handlers::{
    api_keys, auth, charge_points, commands, health, id_tags, monitoring, tariffs, transactions,
};

/// Unified state for all charge-point related routes (CP CRUD + commands + transactions).
/// Axum extracts the specific handler state via `FromRef`.
#[derive(Clone)]
pub struct ChargePointUnifiedState {
    pub storage: Arc<dyn Storage>,
    pub session_registry: SharedSessionRegistry,
    pub command_sender: Arc<CommandSender>,
    pub event_bus: SharedEventBus,
    pub auth: AuthState,
    pub charge_point_service: Arc<ChargePointService>,
}

// -- FromRef implementations so each handler keeps its own State<T> extractor --

impl FromRef<ChargePointUnifiedState> for charge_points::AppState {
    fn from_ref(s: &ChargePointUnifiedState) -> Self {
        charge_points::AppState {
            storage: Arc::clone(&s.storage),
            session_registry: s.session_registry.clone(),
        }
    }
}

impl FromRef<ChargePointUnifiedState> for commands::CommandAppState {
    fn from_ref(s: &ChargePointUnifiedState) -> Self {
        commands::CommandAppState {
            storage: Arc::clone(&s.storage),
            session_registry: s.session_registry.clone(),
            command_sender: Arc::clone(&s.command_sender),
            event_bus: s.event_bus.clone(),
            charge_point_service: Arc::clone(&s.charge_point_service),
        }
    }
}

impl FromRef<ChargePointUnifiedState> for transactions::TransactionAppState {
    fn from_ref(s: &ChargePointUnifiedState) -> Self {
        transactions::TransactionAppState {
            storage: Arc::clone(&s.storage),
        }
    }
}

impl FromRef<ChargePointUnifiedState> for AuthState {
    fn from_ref(s: &ChargePointUnifiedState) -> Self {
        s.auth.clone()
    }
}

/// Security scheme modifier for OpenAPI
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("JWT Bearer token"))
                        .build(),
                ),
            );
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
            );
        }
    }
}

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        // Health
        health::health_check,
        // Auth
        auth::login,
        auth::register,
        auth::get_current_user,
        auth::change_password,
        // API Keys
        api_keys::create_api_key,
        api_keys::list_api_keys,
        api_keys::revoke_api_key,
        // IdTags
        id_tags::list_id_tags,
        id_tags::get_id_tag,
        id_tags::create_id_tag,
        id_tags::update_id_tag,
        id_tags::delete_id_tag,
        id_tags::block_id_tag,
        id_tags::unblock_id_tag,
        // Tariffs
        tariffs::list_tariffs,
        tariffs::get_tariff,
        tariffs::get_default_tariff,
        tariffs::create_tariff,
        tariffs::update_tariff,
        tariffs::delete_tariff,
        tariffs::preview_cost,
        // Charge Points
        charge_points::list_charge_points,
        charge_points::get_charge_point,
        charge_points::delete_charge_point,
        charge_points::get_charge_point_stats,
        charge_points::get_online_charge_points,
        // Connectors
        charge_points::list_connectors,
        charge_points::get_connector,
        charge_points::create_connector,
        charge_points::delete_connector,
        // Monitoring
        monitoring::get_heartbeat_statuses,
        monitoring::get_connection_stats,
        monitoring::get_online_charge_points,
        // Commands
        commands::remote_start,
        commands::remote_stop,
        commands::reset_charge_point,
        commands::unlock,
        commands::change_avail,
        commands::trigger_msg,
        commands::get_config,
        commands::change_config,
        commands::get_local_list_ver,
        commands::clear_auth_cache,
        commands::data_transfer_handler,
        // Transactions
        transactions::list_all_transactions,
        transactions::list_transactions_for_charge_point,
        transactions::get_transaction,
        transactions::get_active_transactions,
        transactions::get_transaction_stats,
        transactions::force_stop_transaction,
    ),
    components(
        schemas(
            // Common
            ApiResponse<String>,
            PaginatedResponse<TransactionDto>,
            PaginatedResponse<id_tags::IdTagDto>,
            PaginationParams,
            // Auth
            auth::LoginRequest,
            auth::LoginResponse,
            auth::RegisterRequest,
            auth::UserInfo,
            auth::ChangePasswordRequest,
            // API Keys
            api_keys::CreateApiKeyRequest,
            api_keys::ApiKeyResponse,
            api_keys::CreatedApiKeyResponse,
            // IdTags
            id_tags::IdTagDto,
            id_tags::CreateIdTagRequest,
            id_tags::UpdateIdTagRequest,
            // Charge Points
            ChargePointDto,
            ConnectorDto,
            ChargePointStats,
            // Transactions
            TransactionDto,
            transactions::TransactionStats,
            // Monitoring
            monitoring::HeartbeatStatusDto,
            monitoring::ConnectionStatsDto,
            // Tariffs
            tariffs::TariffResponse,
            tariffs::CreateTariffRequest,
            tariffs::UpdateTariffRequest,
            tariffs::CostPreviewRequest,
            tariffs::CostBreakdownResponse,
            // Commands
            RemoteStartRequest,
            RemoteStopRequest,
            CreateConnectorRequest,
            ResetRequest,
            UnlockConnectorRequest,
            ChangeAvailabilityRequest,
            TriggerMessageRequest,
            ChangeConfigurationRequest,
            DataTransferRequest,
            DataTransferResponse,
            LocalListVersionResponse,
            CommandResponse,
            commands::ConfigValue,
            commands::ConfigurationResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Server health check endpoints"),
        (name = "Authentication", description = "User authentication: login (JWT), registration, password change"),
        (name = "API Keys", description = "API key management for programmatic access"),
        (name = "IdTags", description = "RFID card and authorization token management (OCPP IdTag)"),
        (name = "Tariffs", description = "Charging tariff management for billing"),
        (name = "Charge Points", description = "Charge point CRUD operations"),
        (name = "Connectors", description = "Charge point connector management"),
        (name = "Monitoring", description = "Real-time monitoring: heartbeat statuses, connection stats"),
        (name = "Commands", description = "OCPP 1.6 remote commands to charge points via WebSocket"),
        (name = "Transactions", description = "Charging session (transaction) management"),
        (name = "WebSocket Notifications", description = "Real-time event notifications via WebSocket"),
    ),
    info(
        title = "Texnouz OCPP Central System API",
        version = "1.0.0",
        description = "REST API for managing OCPP 1.6 Charge Points",
        license(name = "MIT"),
        contact(name = "Texnouz", email = "support@texnouz.com")
    )
)]
pub struct ApiDoc;

/// Create the API router with all routes
pub fn create_api_router(
    storage: Arc<dyn Storage>,
    session_registry: SharedSessionRegistry,
    command_sender: Arc<CommandSender>,
    db: DatabaseConnection,
    jwt_config: JwtConfig,
    heartbeat_monitor: Arc<HeartbeatMonitor>,
    event_bus: SharedEventBus,
    charge_point_service: Arc<ChargePointService>,
) -> Router {
    let middleware_state = AuthState {
        jwt_config: jwt_config.clone(),
        storage: storage.clone(),
        db: db.clone(),
    };

    // ── Unified state for ALL charge-point related routes ───────────
    let cp_unified = ChargePointUnifiedState {
        storage: storage.clone(),
        session_registry: session_registry.clone(),
        command_sender,
        event_bus: event_bus.clone(),
        auth: middleware_state.clone(),
        charge_point_service,
    };

    // A SINGLE router for every /api/v1/charge-points/* route.
    let charge_point_routes = Router::new()
        // --- CP CRUD (uses State<AppState> via FromRef) ---
        .route("/", get(charge_points::list_charge_points))
        .route("/stats", get(charge_points::get_charge_point_stats))
        .route("/online", get(charge_points::get_online_charge_points))
        .route(
            "/{charge_point_id}",
            get(charge_points::get_charge_point).delete(charge_points::delete_charge_point),
        )
        // --- Connectors (uses State<AppState> via FromRef) ---
        .route(
            "/{charge_point_id}/connectors",
            get(charge_points::list_connectors).post(charge_points::create_connector),
        )
        .route(
            "/{charge_point_id}/connectors/{connector_id}",
            get(charge_points::get_connector).delete(charge_points::delete_connector),
        )
        // --- Commands (uses State<CommandAppState> via FromRef) ---
        .route(
            "/{charge_point_id}/remote-start",
            post(commands::remote_start),
        )
        .route(
            "/{charge_point_id}/remote-stop",
            post(commands::remote_stop),
        )
        .route(
            "/{charge_point_id}/reset",
            post(commands::reset_charge_point),
        )
        .route(
            "/{charge_point_id}/unlock-connector",
            post(commands::unlock),
        )
        .route(
            "/{charge_point_id}/change-availability",
            post(commands::change_avail),
        )
        .route(
            "/{charge_point_id}/trigger-message",
            post(commands::trigger_msg),
        )
        .route(
            "/{charge_point_id}/configuration",
            get(commands::get_config).put(commands::change_config),
        )
        .route(
            "/{charge_point_id}/local-list-version",
            get(commands::get_local_list_ver),
        )
        .route(
            "/{charge_point_id}/clear-cache",
            post(commands::clear_auth_cache),
        )
        .route(
            "/{charge_point_id}/data-transfer",
            post(commands::data_transfer_handler),
        )
        // --- Transactions under CP (uses State<TransactionAppState> via FromRef) ---
        .route(
            "/{charge_point_id}/transactions",
            get(transactions::list_transactions_for_charge_point),
        )
        .route(
            "/{charge_point_id}/transactions/active",
            get(transactions::get_active_transactions),
        )
        .route(
            "/{charge_point_id}/transactions/stats",
            get(transactions::get_transaction_stats),
        )
        // auth middleware + unified state
        .layer(middleware::from_fn_with_state(
            middleware_state.clone(),
            auth_middleware,
        ))
        .with_state(cp_unified);

    // ── Other states / routers ─────────────────────────────────

    let auth_state = auth::AuthHandlerState {
        db: db.clone(),
        jwt_config: jwt_config.clone(),
    };

    let api_key_state = api_keys::ApiKeyHandlerState { db: db.clone() };

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Auth routes (public)
    let auth_routes = Router::new()
        .route("/login", post(auth::login))
        .route("/register", post(auth::register))
        .with_state(auth_state.clone());

    // Auth routes (protected)
    let auth_protected_routes = Router::new()
        .route("/me", get(auth::get_current_user))
        .route("/change-password", put(auth::change_password))
        .layer(middleware::from_fn_with_state(
            middleware_state.clone(),
            auth_middleware,
        ))
        .with_state(auth_state);

    // API Key routes (protected)
    let api_key_routes = Router::new()
        .route(
            "/",
            get(api_keys::list_api_keys).post(api_keys::create_api_key),
        )
        .route("/{id}", delete(api_keys::revoke_api_key))
        .layer(middleware::from_fn_with_state(
            middleware_state.clone(),
            auth_middleware,
        ))
        .with_state(api_key_state);

    // IdTag routes (protected)
    let id_tag_state = id_tags::IdTagHandlerState { db: db.clone() };
    let id_tag_routes = Router::new()
        .route("/", get(id_tags::list_id_tags).post(id_tags::create_id_tag))
        .route(
            "/{id_tag}",
            get(id_tags::get_id_tag)
                .put(id_tags::update_id_tag)
                .delete(id_tags::delete_id_tag),
        )
        .route("/{id_tag}/block", post(id_tags::block_id_tag))
        .route("/{id_tag}/unblock", post(id_tags::unblock_id_tag))
        .layer(middleware::from_fn_with_state(
            middleware_state.clone(),
            auth_middleware,
        ))
        .with_state(id_tag_state);

    // Tariff routes (protected)
    let tariff_state = charge_points::AppState {
        storage: storage.clone(),
        session_registry: session_registry.clone(),
    };
    let tariff_routes = Router::new()
        .route("/", get(tariffs::list_tariffs).post(tariffs::create_tariff))
        .route("/default", get(tariffs::get_default_tariff))
        .route("/preview-cost", post(tariffs::preview_cost))
        .route(
            "/{id}",
            get(tariffs::get_tariff)
                .put(tariffs::update_tariff)
                .delete(tariffs::delete_tariff),
        )
        .layer(middleware::from_fn_with_state(
            middleware_state.clone(),
            auth_middleware,
        ))
        .with_state(tariff_state);

    // Transaction routes (standalone, not under charge-points)
    let tx_routes = Router::new()
        .route("/", get(transactions::list_all_transactions))
        .route("/{id}", get(transactions::get_transaction))
        .route(
            "/{transaction_id}/force-stop",
            post(transactions::force_stop_transaction),
        )
        .layer(middleware::from_fn_with_state(
            middleware_state.clone(),
            auth_middleware,
        ))
        .with_state(transactions::TransactionAppState {
            storage: storage.clone(),
        });

    // Monitoring routes (protected)
    let monitoring_state = monitoring::MonitoringState { heartbeat_monitor };
    let monitoring_routes = Router::new()
        .route("/stats", get(monitoring::get_connection_stats))
        .route("/heartbeats", get(monitoring::get_heartbeat_statuses))
        .route("/online", get(monitoring::get_online_charge_points))
        .layer(middleware::from_fn_with_state(
            middleware_state,
            auth_middleware,
        ))
        .with_state(monitoring_state);

    // Notification WebSocket routes (no auth for WebSocket upgrade)
    let notification_state = create_notification_state(event_bus);
    let notification_routes = Router::new()
        .route("/ws", get(ws_notifications_handler))
        .with_state(notification_state);

    let swagger_routes = SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi());

    // Build router
    Router::new()
        // Swagger UI
        .merge(swagger_routes)
        // Health
        .route("/health", get(health::health_check))
        // Auth
        .nest("/api/v1/auth", auth_routes)
        .nest("/api/v1/auth", auth_protected_routes)
        // API Keys
        .nest("/api/v1/api-keys", api_key_routes)
        // IdTags
        .nest("/api/v1/id-tags", id_tag_routes)
        // Tariffs
        .nest("/api/v1/tariffs", tariff_routes)
        // Charge Points
        .nest("/api/v1/charge-points", charge_point_routes)
        // Transactions (standalone)
        .nest("/api/v1/transactions", tx_routes)
        // Monitoring
        .nest("/api/v1/monitoring", monitoring_routes)
        // Notifications WebSocket
        .nest("/api/v1/notifications", notification_routes)
        // Middleware
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
