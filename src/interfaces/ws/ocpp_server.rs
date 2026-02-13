//! Unified OCPP WebSocket server
//!
//! Accepts charge-point connections at `ws://<host>:<port>/ocpp/{charge_point_id}`.
//! During the WebSocket handshake the server negotiates the OCPP version
//! via the `Sec-WebSocket-Protocol` header and creates the appropriate
//! version-specific adapter for each connection.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use chrono::Utc;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

use crate::application::events::{
    ChargePointConnectedEvent, ChargePointDisconnectedEvent, Event, SharedEventBus,
};
use crate::application::SharedCommandSender;
use crate::application::SharedSessionRegistry;
use crate::application::session::RegisterResult;
use crate::config::Config;
use crate::domain::RepositoryProvider;
use crate::domain::OcppVersion;
use crate::shared::shutdown::ShutdownSignal;

use super::negotiator::ProtocolAdapters;

/// Unified OCPP WebSocket Server
///
/// Supports multiple OCPP versions through protocol adapters.
/// During handshake, the server negotiates the best OCPP version
/// with each charge point and creates the appropriate adapter.
pub struct OcppServer {
    config: Config,
    protocol_adapters: Arc<ProtocolAdapters>,
    session_registry: SharedSessionRegistry,
    command_sender: SharedCommandSender,
    shutdown_signal: Option<ShutdownSignal>,
    event_bus: SharedEventBus,
    repos: Arc<dyn RepositoryProvider>,
    /// Per-IP WebSocket connection rate limiter: max connections per minute
    ws_rate_limiter: Arc<WsRateLimiter>,
}

impl OcppServer {
    /// Create a new unified OCPP WebSocket server.
    ///
    /// Dependencies are injected — the session registry, command sender,
    /// and protocol adapters are created externally (in `main`).
    pub fn new(
        config: Config,
        protocol_adapters: Arc<ProtocolAdapters>,
        session_registry: SharedSessionRegistry,
        command_sender: SharedCommandSender,
        event_bus: SharedEventBus,
        repos: Arc<dyn RepositoryProvider>,
        ws_connections_per_minute: u32,
    ) -> Self {
        Self {
            config,
            protocol_adapters,
            session_registry,
            command_sender,
            shutdown_signal: None,
            event_bus,
            repos,
            ws_rate_limiter: Arc::new(WsRateLimiter::new(ws_connections_per_minute)),
        }
    }

    /// Set the shutdown signal for graceful shutdown
    pub fn with_shutdown(mut self, signal: ShutdownSignal) -> Self {
        self.shutdown_signal = Some(signal);
        self
    }

    /// Start the WebSocket server
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = self.config.address();
        let listener = TcpListener::bind(&addr).await?;

        let negotiator = self.protocol_adapters.build_negotiator();
        let supported: Vec<String> = negotiator
            .supported_versions()
            .iter()
            .map(|v| v.to_string())
            .collect();

        info!("🔌 OCPP Central System started on ws://{}", addr);
        info!("   Supported protocols: {}", supported.join(", "));
        info!(
            "   Charge points should connect to: ws://{}/ocpp/{{charge_point_id}}",
            addr
        );

        if let Some(ref shutdown) = self.shutdown_signal {
            self.run_with_shutdown(listener, shutdown.clone()).await
        } else {
            self.run_loop(listener).await
        }
    }

    async fn run_loop(
        &self,
        listener: TcpListener,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        while let Ok((stream, addr)) = listener.accept().await {
            self.spawn_connection(stream, addr);
        }
        Ok(())
    }

    async fn run_with_shutdown(
        &self,
        listener: TcpListener,
        shutdown: ShutdownSignal,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            self.spawn_connection(stream, addr);
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }
                _ = shutdown.notified().wait() => {
                    info!("🛑 WebSocket server received shutdown signal");
                    self.graceful_shutdown().await;
                    return Ok(());
                }
            }
        }
    }

    fn spawn_connection(&self, stream: TcpStream, addr: SocketAddr) {
        // WS rate limiting: check per-IP connection rate
        if !self.ws_rate_limiter.check(addr.ip()) {
            warn!(
                "🛡️ WS rate limit exceeded for IP {}, dropping connection",
                addr.ip()
            );
            // Just drop the stream — the TCP connection will be closed
            return;
        }

        let session_registry = self.session_registry.clone();
        let protocol_adapters = self.protocol_adapters.clone();
        let command_sender = self.command_sender.clone();
        let shutdown = self.shutdown_signal.clone();
        let event_bus = self.event_bus.clone();
        let repos = self.repos.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(
                stream,
                addr,
                session_registry,
                protocol_adapters,
                command_sender,
                shutdown,
                event_bus,
                repos,
            )
            .await
            {
                error!("Connection error from {}: {}", addr, e);
            }
        });
    }

    async fn graceful_shutdown(&self) {
        let connected = self.session_registry.connected_ids();
        let count = connected.len();

        if count > 0 {
            info!(
                "📢 Notifying {} connected charge points about shutdown...",
                count
            );
            for cp_id in &connected {
                info!("  → Closing connection to {}", cp_id);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        for cp_id in connected {
            self.session_registry.unregister(&cp_id);
        }

        info!("✅ WebSocket server shutdown complete");
    }
}

/// Extract charge point ID from WebSocket request path.
/// Expected format: /ocpp/{charge_point_id} or /{charge_point_id}
fn extract_charge_point_id(path: &str) -> Option<String> {
    let path = path.trim_start_matches('/');

    if let Some(id) = path.strip_prefix("ocpp/") {
        let id = id.trim_start_matches('/');
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }

    if !path.is_empty() && !path.contains('/') {
        return Some(path.to_string());
    }

    None
}

/// Handle a single WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    session_registry: SharedSessionRegistry,
    protocol_adapters: Arc<ProtocolAdapters>,
    command_sender: SharedCommandSender,
    shutdown: Option<ShutdownSignal>,
    event_bus: SharedEventBus,
    repos: Arc<dyn RepositoryProvider>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("New connection from: {}", addr);

    let mut charge_point_id: Option<String> = None;
    let mut negotiated_version: Option<OcppVersion> = None;

    let negotiator = protocol_adapters.build_negotiator();

    let ws_stream =
        tokio_tungstenite::accept_hdr_async(stream, |req: &Request, mut response: Response| {
            let path = req.uri().path();
            info!("WebSocket handshake from: {}, path: {}", addr, path);

            let requested_protocols = req
                .headers()
                .get("Sec-WebSocket-Protocol")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            info!("Requested subprotocols: {}", requested_protocols);

            // ── Version negotiation ────────────────────────
            if let Some(version) = negotiator.negotiate(requested_protocols) {
                response.headers_mut().insert(
                    "Sec-WebSocket-Protocol",
                    version.subprotocol().parse().unwrap(),
                );
                info!("{} subprotocol accepted", version);
                negotiated_version = Some(version);
            } else if !requested_protocols.is_empty() {
                warn!(
                    "No mutually supported OCPP subprotocol, requested: {}",
                    requested_protocols
                );
                // Fall back to the lowest version we support (V16 if available)
                negotiated_version = negotiator.supported_versions().last().copied();
                if let Some(fallback) = negotiated_version {
                    warn!("Falling back to {} (no subprotocol negotiated)", fallback);
                }
            } else {
                // No subprotocol header at all — default to V16
                negotiated_version = Some(OcppVersion::V16);
            }

            // ── Extract charge point ID from path ──────────
            if let Some(id) = extract_charge_point_id(path) {
                charge_point_id = Some(id);
                Ok(response)
            } else {
                charge_point_id = Some(format!("CP_{}", addr.port()));
                Ok(response)
            }
        })
        .await?;

    let charge_point_id = charge_point_id.unwrap_or_else(|| format!("CP_{}", addr.port()));
    let version = negotiated_version.unwrap_or(OcppVersion::V16);

    info!(
        "[{}] Connected from {} via {}",
        charge_point_id, addr, version
    );

    // ── Whitelist check: verify charge_point_id exists in DB ──
    match repos.charge_points().find_by_id(&charge_point_id).await {
        Ok(Some(_)) => {
            info!(
                "[{}] Charge point found in database, connection allowed",
                charge_point_id
            );
        }
        Ok(None) => {
            warn!(
                "[{}] Charge point NOT found in database — rejecting connection from {}",
                charge_point_id, addr
            );
            // Send close frame and drop connection
            let (mut ws_sender, _) = ws_stream.split();
            let close_frame = tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Policy,
                reason: "Unknown charge point ID".into(),
            };
            let _ = ws_sender
                .send(Message::Close(Some(close_frame)))
                .await;
            return Ok(());
        }
        Err(e) => {
            error!(
                "[{}] Failed to verify charge point in database: {} — rejecting connection",
                charge_point_id, e
            );
            let (mut ws_sender, _) = ws_stream.split();
            let close_frame = tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Error,
                reason: "Internal server error during authentication".into(),
            };
            let _ = ws_sender
                .send(Message::Close(Some(close_frame)))
                .await;
            return Ok(());
        }
    }

    // ── Create version-specific adapter ────────────────────
    let adapter = protocol_adapters
        .create_adapter(version, charge_point_id.clone())
        .ok_or_else(|| {
            format!(
                "No adapter registered for {} — cannot handle connection",
                version
            )
        })?;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // ── Register session (with eviction / debounce) ────────
    match session_registry.register(&charge_point_id, tx, version) {
        RegisterResult::Debounced {
            seconds_remaining, ..
        } => {
            warn!(
                "[{}] Reconnection rejected — debounce active, retry in {}s",
                charge_point_id, seconds_remaining
            );
            let close_frame = tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Again,
                reason: format!("Reconnecting too fast, retry in {}s", seconds_remaining).into(),
            };
            let _ = ws_sender.send(Message::Close(Some(close_frame))).await;
            return Ok(());
        }
        RegisterResult::Evicted(old) => {
            warn!(
                "[{}] Evicted previous session (was {} since {}, last active {})",
                charge_point_id, old.ocpp_version, old.connected_at, old.last_activity
            );
            // Old sender was dropped → old send_task will exit.
            // Clean up pending commands for the old session.
            command_sender.cleanup_charge_point(&charge_point_id);
            // Publish disconnect event for the evicted session.
            event_bus.publish(Event::ChargePointDisconnected(
                ChargePointDisconnectedEvent {
                    charge_point_id: charge_point_id.clone(),
                    timestamp: Utc::now(),
                    reason: Some("Evicted by new connection".to_string()),
                },
            ));
        }
        RegisterResult::New => {
            // First connection — nothing to clean up
        }
    }

    event_bus.publish(Event::ChargePointConnected(ChargePointConnectedEvent {
        charge_point_id: charge_point_id.clone(),
        ocpp_version: version.version_string().to_string(),
        timestamp: Utc::now(),
        remote_addr: Some(addr.to_string()),
    }));

    // Outgoing message sender task
    let cp_id_send = charge_point_id.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            info!("[{}] -> {}", cp_id_send, msg);
            metrics::counter!("ws_messages_total", "direction" => "outbound", "type" => "text").increment(1);
            if let Err(e) = ws_sender.send(Message::Text(msg)).await {
                error!("[{}] Send error: {}", cp_id_send, e);
                break;
            }
        }
    });

    // Incoming message receiver task
    let cp_id_recv = charge_point_id.clone();
    let session_reg = session_registry.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("[{}] <- {}", cp_id_recv, text);
                    metrics::counter!("ws_messages_total", "direction" => "inbound", "type" => "text").increment(1);
                    session_reg.touch(&cp_id_recv);

                    // Dispatch to the version-specific adapter
                    if let Some(response) = adapter.handle_message(&text).await {
                        if let Err(e) = session_reg.send_to(&cp_id_recv, response) {
                            error!("[{}] Failed to send response: {}", cp_id_recv, e);
                            break;
                        }
                    }
                }
                Ok(Message::Ping(_)) => {
                    info!("[{}] Ping received", cp_id_recv);
                    metrics::counter!("ws_messages_total", "direction" => "inbound", "type" => "ping").increment(1);
                }
                Ok(Message::Pong(_)) => {
                    info!("[{}] Pong received", cp_id_recv);
                    metrics::counter!("ws_messages_total", "direction" => "inbound", "type" => "pong").increment(1);
                }
                Ok(Message::Close(frame)) => {
                    info!("[{}] Close frame received: {:?}", cp_id_recv, frame);
                    metrics::counter!("ws_messages_total", "direction" => "inbound", "type" => "close").increment(1);
                    break;
                }
                Ok(Message::Binary(data)) => {
                    warn!(
                        "[{}] Binary message received ({} bytes), ignoring",
                        cp_id_recv,
                        data.len()
                    );
                    metrics::counter!("ws_messages_total", "direction" => "inbound", "type" => "binary").increment(1);
                }
                Ok(Message::Frame(_)) => {}
                Err(e) => {
                    error!("[{}] WebSocket error: {}", cp_id_recv, e);
                    break;
                }
            }
        }

        session_reg.unregister(&cp_id_recv);
    });

    // Wait for tasks or shutdown
    if let Some(shutdown) = shutdown {
        tokio::select! {
            _ = send_task => {},
            _ = recv_task => {},
            _ = shutdown.notified().wait() => {
                info!("[{}] Connection closing due to server shutdown", charge_point_id);
            }
        }
    } else {
        tokio::select! {
            _ = send_task => {},
            _ = recv_task => {},
        }
    }

    // Cleanup
    session_registry.unregister(&charge_point_id);
    command_sender.cleanup_charge_point(&charge_point_id);

    event_bus.publish(Event::ChargePointDisconnected(
        ChargePointDisconnectedEvent {
            charge_point_id: charge_point_id.clone(),
            timestamp: Utc::now(),
            reason: None,
        },
    ));

    info!("[{}] Disconnected", charge_point_id);

    Ok(())
}

// ── WebSocket connection rate limiter ──────────────────────────

/// Simple per-IP sliding-window rate limiter for WebSocket connections.
///
/// Tracks connection timestamps per IP and rejects new connections
/// if the count within the last 60 seconds exceeds the configured limit.
struct WsRateLimiter {
    /// Map from IP address to list of connection timestamps within the window
    connections: DashMap<IpAddr, Vec<Instant>>,
    /// Maximum connections per minute per IP
    max_per_minute: u32,
}

impl WsRateLimiter {
    fn new(max_per_minute: u32) -> Self {
        Self {
            connections: DashMap::new(),
            max_per_minute,
        }
    }

    /// Check if a new connection from this IP is allowed.
    /// Returns `true` if allowed, `false` if rate-limited.
    fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(60);

        let mut entry = self.connections.entry(ip).or_default();
        // Remove timestamps older than the window
        entry.retain(|t| now.duration_since(*t) < window);

        if entry.len() >= self.max_per_minute as usize {
            false
        } else {
            entry.push(now);
            true
        }
    }
}
