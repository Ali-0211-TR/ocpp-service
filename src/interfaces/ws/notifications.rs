//! WebSocket handler for UI notification clients
//!
//! Provides real-time event streaming to UI clients.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::select;
use tracing::{debug, error, info, warn};

use crate::application::events::{EventMessage, SharedEventBus};

/// Query parameters for filtering events
#[derive(Debug, Deserialize)]
pub struct EventFilter {
    /// Filter by charge point ID (optional)
    pub charge_point_id: Option<String>,
    /// Filter by event types (comma-separated, optional)
    pub event_types: Option<String>,
}

impl EventFilter {
    /// Check if event matches the filter
    pub fn matches(&self, event: &EventMessage) -> bool {
        if let Some(ref cp_id) = self.charge_point_id {
            if let Some(event_cp_id) = event.event.charge_point_id() {
                if event_cp_id != cp_id {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref types) = self.event_types {
            let allowed_types: Vec<&str> = types.split(',').map(|s| s.trim()).collect();
            if !allowed_types.contains(&event.event.event_type()) {
                return false;
            }
        }

        true
    }
}

/// State for notification WebSocket handler
#[derive(Clone)]
pub struct NotificationState {
    pub event_bus: SharedEventBus,
}

/// WebSocket upgrade handler for notifications
pub async fn ws_notifications_handler(
    ws: WebSocketUpgrade,
    State(state): State<NotificationState>,
    Query(filter): Query<EventFilter>,
) -> impl IntoResponse {
    info!(
        "New notification WebSocket connection: charge_point={:?}, event_types={:?}",
        filter.charge_point_id, filter.event_types
    );

    ws.on_upgrade(move |socket| handle_notification_socket(socket, state, filter))
}

/// Handle a WebSocket connection for notifications
async fn handle_notification_socket(
    socket: WebSocket,
    state: NotificationState,
    filter: EventFilter,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut subscriber = state.event_bus.subscribe();

    // Send welcome message
    let welcome = serde_json::json!({
        "type": "connected",
        "message": "Connected to notification stream",
        "filter": {
            "charge_point_id": filter.charge_point_id,
            "event_types": filter.event_types
        }
    });

    if let Err(e) = sender
        .send(Message::Text(welcome.to_string().into()))
        .await
    {
        error!("Failed to send welcome message: {}", e);
        return;
    }

    info!("Notification WebSocket client connected");

    loop {
        select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        debug!("Received text message: {}", text);
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if let Err(e) = sender.send(Message::Pong(data)).await {
                            error!("Failed to send pong: {}", e);
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        debug!("Received pong");
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Client sent close");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }

            event = subscriber.recv() => {
                match event {
                    Some(event_msg) => {
                        if !filter.matches(&event_msg) {
                            continue;
                        }

                        match serde_json::to_string(&event_msg) {
                            Ok(json) => {
                                if let Err(e) = sender.send(Message::Text(json.into())).await {
                                    error!("Failed to send event: {}", e);
                                    break;
                                }
                                debug!("Event sent to client: {}", event_msg.event.event_type());
                            }
                            Err(e) => {
                                error!("Failed to serialize event: {}", e);
                            }
                        }
                    }
                    None => {
                        warn!("Event bus closed");
                        break;
                    }
                }
            }
        }
    }

    info!("Notification WebSocket client disconnected");
}

/// Create notification state
pub fn create_notification_state(event_bus: SharedEventBus) -> NotificationState {
    NotificationState { event_bus }
}
