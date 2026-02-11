//! OCPP 1.6 WebSocket server
//!
//! Accepts charge-point connections at `ws://<host>:<port>/ocpp/{charge_point_id}`.

use std::net::SocketAddr;
use std::sync::Arc;

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

use crate::application::events::{
    ChargePointConnectedEvent, ChargePointDisconnectedEvent, Event, SharedEventBus,
};
use crate::application::handlers::OcppHandler;
use crate::application::services::{BillingService, ChargePointService};
use crate::application::commands::{SharedCommandSender, create_command_sender};
use crate::application::session::{SharedSessionRegistry, SessionRegistry};
use crate::config::Config;
use crate::support::shutdown::ShutdownSignal;

/// OCPP 1.6 WebSocket subprotocol
const OCPP_SUBPROTOCOL: &str = "ocpp1.6";

/// OCPP WebSocket Server
pub struct OcppServer {
    config: Config,
    session_registry: SharedSessionRegistry,
    service: Arc<ChargePointService>,
    billing_service: Arc<BillingService>,
    command_sender: SharedCommandSender,
    shutdown_signal: Option<ShutdownSignal>,
    event_bus: SharedEventBus,
}

impl OcppServer {
    pub fn new(
        config: Config,
        service: Arc<ChargePointService>,
        billing_service: Arc<BillingService>,
        event_bus: SharedEventBus,
    ) -> Self {
        let session_registry = Arc::new(SessionRegistry::new());
        let command_sender = create_command_sender(session_registry.clone());

        Self {
            config,
            session_registry,
            service,
            billing_service,
            command_sender,
            shutdown_signal: None,
            event_bus,
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

        info!("🔌 OCPP 1.6 Central System started on ws://{}", addr);
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
        let session_registry = self.session_registry.clone();
        let service = self.service.clone();
        let billing_service = self.billing_service.clone();
        let command_sender = self.command_sender.clone();
        let shutdown = self.shutdown_signal.clone();
        let event_bus = self.event_bus.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(
                stream,
                addr,
                session_registry,
                service,
                billing_service,
                command_sender,
                shutdown,
                event_bus,
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

    pub fn session_registry(&self) -> &SharedSessionRegistry {
        &self.session_registry
    }

    pub fn get_session_registry(&self) -> SharedSessionRegistry {
        self.session_registry.clone()
    }

    pub fn command_sender(&self) -> &SharedCommandSender {
        &self.command_sender
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
    service: Arc<ChargePointService>,
    billing_service: Arc<BillingService>,
    command_sender: SharedCommandSender,
    shutdown: Option<ShutdownSignal>,
    event_bus: SharedEventBus,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("New connection from: {}", addr);

    let mut charge_point_id: Option<String> = None;

    let ws_stream = tokio_tungstenite::accept_hdr_async(
        stream,
        |req: &Request, mut response: Response| {
            let path = req.uri().path();
            info!("WebSocket handshake from: {}, path: {}", addr, path);

            let requested_protocols = req
                .headers()
                .get("Sec-WebSocket-Protocol")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            info!("Requested subprotocols: {}", requested_protocols);

            let supports_ocpp16 = requested_protocols
                .split(',')
                .map(|s| s.trim())
                .any(|p| p == OCPP_SUBPROTOCOL);

            if supports_ocpp16 {
                response.headers_mut().insert(
                    "Sec-WebSocket-Protocol",
                    OCPP_SUBPROTOCOL.parse().unwrap(),
                );
                info!("OCPP 1.6 subprotocol accepted");
            } else if !requested_protocols.is_empty() {
                warn!(
                    "Client does not support ocpp1.6, requested: {}",
                    requested_protocols
                );
            }

            if let Some(id) = extract_charge_point_id(path) {
                charge_point_id = Some(id);
                Ok(response)
            } else {
                charge_point_id = Some(format!("CP_{}", addr.port()));
                Ok(response)
            }
        },
    )
    .await?;

    let charge_point_id = charge_point_id.unwrap_or_else(|| format!("CP_{}", addr.port()));

    info!("[{}] Connected from {}", charge_point_id, addr);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    session_registry.register(&charge_point_id, tx);

    event_bus.publish(Event::ChargePointConnected(ChargePointConnectedEvent {
        charge_point_id: charge_point_id.clone(),
        timestamp: Utc::now(),
        remote_addr: Some(addr.to_string()),
    }));

    let handler = Arc::new(OcppHandler::new(
        charge_point_id.clone(),
        service,
        billing_service,
        command_sender.clone(),
        event_bus.clone(),
    ));

    // Outgoing message sender task
    let cp_id_send = charge_point_id.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            info!("[{}] -> {}", cp_id_send, msg);
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
                    session_reg.touch(&cp_id_recv);

                    if let Some(response) = handler.handle(&text).await {
                        if let Err(e) = session_reg.send_to(&cp_id_recv, response) {
                            error!("[{}] Failed to send response: {}", cp_id_recv, e);
                            break;
                        }
                    }
                }
                Ok(Message::Ping(_)) => {
                    info!("[{}] Ping received", cp_id_recv);
                }
                Ok(Message::Pong(_)) => {
                    info!("[{}] Pong received", cp_id_recv);
                }
                Ok(Message::Close(frame)) => {
                    info!("[{}] Close frame received: {:?}", cp_id_recv, frame);
                    break;
                }
                Ok(Message::Binary(data)) => {
                    warn!(
                        "[{}] Binary message received ({} bytes), ignoring",
                        cp_id_recv,
                        data.len()
                    );
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
