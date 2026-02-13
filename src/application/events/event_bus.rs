//! Event Bus for broadcasting events to subscribers

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::domain::events::{Event, EventMessage};

const DEFAULT_CAPACITY: usize = 1024;

/// Event bus for broadcasting events to all subscribers
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<EventMessage>,
    subscriber_count: Arc<AtomicUsize>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            subscriber_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn publish(&self, event: Event) {
        let message = EventMessage::new(event);
        let event_type = message.event.event_type();
        let charge_point_id = message.event.charge_point_id().map(String::from);

        // Record event metrics
        metrics::counter!("ocpp_events_total", "type" => event_type).increment(1);

        // Track transaction lifecycle specifically
        match event_type {
            "transaction_started" => {
                metrics::counter!("ocpp_transactions_total", "status" => "started").increment(1);
            }
            "transaction_stopped" => {
                metrics::counter!("ocpp_transactions_total", "status" => "stopped").increment(1);
            }
            "transaction_billed" => {
                metrics::counter!("ocpp_transactions_total", "status" => "billed").increment(1);
            }
            _ => {}
        }

        match self.sender.send(message) {
            Ok(count) => {
                debug!(
                    event_type,
                    ?charge_point_id,
                    subscribers = count,
                    "Event published"
                );
            }
            Err(_) => {
                debug!(
                    event_type,
                    ?charge_point_id,
                    "Event published (no subscribers)"
                );
            }
        }
    }

    pub fn subscribe(&self) -> EventSubscriber {
        let receiver = self.sender.subscribe();
        self.subscriber_count.fetch_add(1, Ordering::SeqCst);
        let count = self.subscriber_count.load(Ordering::SeqCst);
        info!(total = count, "New event subscriber");

        EventSubscriber {
            receiver,
            subscriber_count: self.subscriber_count.clone(),
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscriber_count.load(Ordering::SeqCst)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Event subscriber that receives events from the bus
pub struct EventSubscriber {
    receiver: broadcast::Receiver<EventMessage>,
    subscriber_count: Arc<AtomicUsize>,
}

impl EventSubscriber {
    pub async fn recv(&mut self) -> Option<EventMessage> {
        loop {
            match self.receiver.recv().await {
                Ok(msg) => return Some(msg),
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    warn!(missed = count, "Subscriber lagged");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return None;
                }
            }
        }
    }
}

impl Drop for EventSubscriber {
    fn drop(&mut self) {
        let prev = self.subscriber_count.fetch_sub(1, Ordering::SeqCst);
        info!(remaining = prev - 1, "Event subscriber disconnected");
    }
}

/// Shared event bus type
pub type SharedEventBus = Arc<EventBus>;

/// Create a shared event bus
pub fn create_event_bus() -> SharedEventBus {
    Arc::new(EventBus::new())
}
