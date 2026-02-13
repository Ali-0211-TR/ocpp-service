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

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::{Event, HeartbeatEvent};

    fn make_event() -> Event {
        Event::HeartbeatReceived(HeartbeatEvent {
            charge_point_id: "CP001".into(),
            timestamp: chrono::Utc::now(),
        })
    }

    #[test]
    fn new_event_bus_has_no_subscribers() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn subscribe_increments_count() {
        let bus = EventBus::new();
        let _s1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);
        let _s2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[test]
    fn drop_subscriber_decrements_count() {
        let bus = EventBus::new();
        let s1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);
        drop(s1);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn publish_and_receive() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();

        bus.publish(make_event());

        let msg = tokio::time::timeout(std::time::Duration::from_secs(1), sub.recv())
            .await
            .expect("timed out")
            .expect("channel closed");

        assert_eq!(msg.event.event_type(), "heartbeat_received");
        assert_eq!(msg.event.charge_point_id(), Some("CP001"));
    }

    #[tokio::test]
    async fn publish_without_subscribers_does_not_panic() {
        let bus = EventBus::new();
        bus.publish(make_event()); // no subscriber — should not panic
    }

    #[test]
    fn with_capacity_creates_bus() {
        let bus = EventBus::with_capacity(16);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn default_creates_bus() {
        let bus = EventBus::default();
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn create_event_bus_returns_arc() {
        let bus = create_event_bus();
        assert_eq!(bus.subscriber_count(), 0);
    }
}
