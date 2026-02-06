//! WebSocket server for OCPP connections

use std::net::SocketAddr;
use std::sync::Arc;

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;

use crate::application::services::{BillingService, ChargePointService};
use crate::application::{CommandSender, OcppHandler};
use crate::config::Config;
use crate::notifications::{
    ChargePointConnectedEvent, ChargePointDisconnectedEvent, Event, SharedEventBus,
};
use crate::session::{create_session_manager, SharedSessionManager};

use super::shutdown::ShutdownSignal;

/// OCPP 1.6 WebSocket subprotocol
const OCPP_SUBPROTOCOL: &str = "ocpp1.6";

/// OCPP WebSocket Server
pub struct OcppServer {
    config: Config,
    session_manager: SharedSessionManager,
    service: Arc<ChargePointService>,
    billing_service: Arc<BillingService>,
    command_sender: Arc<CommandSender>,
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
        let session_manager = create_session_manager();
        let command_sender = Arc::new(CommandSender::new(session_manager.clone()));

        Self {
            config,
            session_manager,
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

        // If we have a shutdown signal, use it
        if let Some(ref shutdown) = self.shutdown_signal {
            self.run_with_shutdown(listener, shutdown.clone()).await
        } else {
            self.run_loop(listener).await
        }
    }

    /// Run the server loop without shutdown support
    async fn run_loop(&self, listener: TcpListener) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        while let Ok((stream, addr)) = listener.accept().await {
            self.spawn_connection(stream, addr);
        }
        Ok(())
    }

    /// Run the server loop with shutdown support
    async fn run_with_shutdown(
        &self,
        listener: TcpListener,
        shutdown: ShutdownSignal,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            tokio::select! {
                // Accept new connections
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
                // Handle shutdown signal
                _ = shutdown.notified().wait() => {
                    info!("🛑 WebSocket server received shutdown signal");
                    self.graceful_shutdown().await;
                    return Ok(());
                }
            }
        }
    }

    /// Spawn a connection handler task
    fn spawn_connection(&self, stream: TcpStream, addr: SocketAddr) {
        let session_manager = self.session_manager.clone();
        let service = self.service.clone();
        let billing_service = self.billing_service.clone();
        let command_sender = self.command_sender.clone();
        let shutdown = self.shutdown_signal.clone();
        let event_bus = self.event_bus.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(
                stream,
                addr,
                session_manager,
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

    /// Perform graceful shutdown
    async fn graceful_shutdown(&self) {
        let connected = self.session_manager.connected_ids();
        let count = connected.len();
        
        if count > 0 {
            info!("📢 Notifying {} connected charge points about shutdown...", count);
            
            // Send close messages to all connected charge points
            // In OCPP, there's no specific "server shutdown" message,
            // but we can close WebSocket connections gracefully
            for cp_id in &connected {
                info!("  → Closing connection to {}", cp_id);
                // The connection tasks will handle the actual close
            }
        }
        
        // Wait a bit for connections to close gracefully
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Unregister all remaining sessions
        for cp_id in connected {
            self.session_manager.unregister(&cp_id);
        }
        
        info!("✅ WebSocket server shutdown complete");
    }

    /// Get the session manager
    pub fn session_manager(&self) -> &SharedSessionManager {
        &self.session_manager
    }

    /// Get the session manager (cloned for sharing)
    pub fn get_session_manager(&self) -> SharedSessionManager {
        self.session_manager.clone()
    }

    /// Get the command sender for sending commands to charge points
    pub fn command_sender(&self) -> &Arc<CommandSender> {
        &self.command_sender
    }
}

/// Extract charge point ID from WebSocket request path
/// Expected format: /ocpp/{charge_point_id} or /{charge_point_id}
fn extract_charge_point_id(path: &str) -> Option<String> {
    let path = path.trim_start_matches('/');

    // Try /ocpp/{id} format first
    if let Some(id) = path.strip_prefix("ocpp/") {
        let id = id.trim_start_matches('/');
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }

    // Fallback: use path directly as ID if it doesn't contain '/'
    if !path.is_empty() && !path.contains('/') {
        return Some(path.to_string());
    }

    None
}

/// Handle a WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    session_manager: SharedSessionManager,
    service: Arc<ChargePointService>,
    billing_service: Arc<BillingService>,
    command_sender: Arc<CommandSender>,
    shutdown: Option<ShutdownSignal>,
    event_bus: SharedEventBus,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("New connection from: {}", addr);

    // Track the charge point ID from the handshake
    let mut charge_point_id: Option<String> = None;

    // Accept WebSocket connection with OCPP subprotocol negotiation
    let ws_stream = tokio_tungstenite::accept_hdr_async(stream, |req: &Request, mut response: Response| {
        let path = req.uri().path();
        info!("WebSocket handshake from: {}, path: {}", addr, path);

        // Check for OCPP subprotocol in Sec-WebSocket-Protocol header
        let requested_protocols = req
            .headers()
            .get("Sec-WebSocket-Protocol")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        info!("Requested subprotocols: {}", requested_protocols);

        // Check if client supports ocpp1.6
        let supports_ocpp16 = requested_protocols
            .split(',')
            .map(|s| s.trim())
            .any(|p| p == OCPP_SUBPROTOCOL);

        if supports_ocpp16 {
            // Set the accepted subprotocol in response
            response.headers_mut().insert(
                "Sec-WebSocket-Protocol",
                OCPP_SUBPROTOCOL.parse().unwrap(),
            );
            info!("OCPP 1.6 subprotocol accepted");
        } else if !requested_protocols.is_empty() {
            // Client requested protocols but not ocpp1.6
            warn!(
                "Client does not support ocpp1.6, requested: {}",
                requested_protocols
            );
            // Still accept connection for testing purposes
        }

        // Extract charge point ID from path
        if let Some(id) = extract_charge_point_id(path) {
            charge_point_id = Some(id);
            Ok(response)
        } else {
            // If no ID in path, generate one from address
            charge_point_id = Some(format!("CP_{}", addr.port()));
            Ok(response)
        }
    })
    .await?;

    // This is a workaround since we can't easily get the ID from the closure
    let charge_point_id = charge_point_id.unwrap_or_else(|| format!("CP_{}", addr.port()));

    info!("[{}] Connected from {}", charge_point_id, addr);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Create channel for sending messages to charge point
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Register session
    session_manager.register(charge_point_id.clone(), tx);

    // Publish connected event
    event_bus.publish(Event::ChargePointConnected(ChargePointConnectedEvent {
        charge_point_id: charge_point_id.clone(),
        timestamp: Utc::now(),
        remote_addr: Some(addr.to_string()),
    }));

    // Create handler
    let handler = Arc::new(OcppHandler::new(
        charge_point_id.clone(),
        service,
        billing_service,
        command_sender.clone(),
        event_bus.clone(),
    ));

    // Spawn task to handle outgoing messages
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

    // Handle incoming messages
    let cp_id_recv = charge_point_id.clone();
    let session_mgr = session_manager.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("[{}] <- {}", cp_id_recv, text);
                    session_mgr.touch(&cp_id_recv);

                    if let Some(response) = handler.handle(&text).await {
                        if let Err(e) = session_mgr.send_to(&cp_id_recv, response) {
                            error!("[{}] Failed to send response: {}", cp_id_recv, e);
                            break;
                        }
                    }
                }
                Ok(Message::Ping(_)) => {
                    info!("[{}] Ping received", cp_id_recv);
                    // Pong is sent automatically by tungstenite
                }
                Ok(Message::Pong(_)) => {
                    info!("[{}] Pong received", cp_id_recv);
                }
                Ok(Message::Close(frame)) => {
                    info!("[{}] Close frame received: {:?}", cp_id_recv, frame);
                    break;
                }
                Ok(Message::Binary(data)) => {
                    warn!("[{}] Binary message received ({} bytes), ignoring", cp_id_recv, data.len());
                }
                Ok(Message::Frame(_)) => {
                    // Raw frame, ignore
                }
                Err(e) => {
                    error!("[{}] WebSocket error: {}", cp_id_recv, e);
                    break;
                }
            }
        }

        // Unregister session when connection closes
        session_mgr.unregister(&cp_id_recv);
    });

    // Wait for tasks or shutdown signal
    if let Some(shutdown) = shutdown {
        tokio::select! {
            _ = send_task => {},
            _ = recv_task => {},
            _ = shutdown.notified().wait() => {
                info!("[{}] Connection closing due to server shutdown", charge_point_id);
            }
        }
    } else {
        // Wait for either task to complete
        tokio::select! {
            _ = send_task => {},
            _ = recv_task => {},
        }
    }

    // Clean up
    session_manager.unregister(&charge_point_id);
    command_sender.cleanup_charge_point(&charge_point_id);
    
    // Publish disconnected event
    event_bus.publish(Event::ChargePointDisconnected(ChargePointDisconnectedEvent {
        charge_point_id: charge_point_id.clone(),
        timestamp: Utc::now(),
        reason: None,
    }));
    
    info!("[{}] Disconnected", charge_point_id);

    Ok(())
}
