//! Event Bus for broadcasting events to subscribers
//!
//! Uses tokio broadcast channel for pub/sub pattern.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use log::{debug, info, warn};
use tokio::sync::broadcast;

use super::events::{Event, EventMessage};

/// Default channel capacity
const DEFAULT_CAPACITY: usize = 1024;

/// Event bus for broadcasting events to all subscribers
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<EventMessage>,
    subscriber_count: Arc<AtomicUsize>,
}

impl EventBus {
    /// Create a new event bus with default capacity
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a new event bus with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            subscriber_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: Event) {
        let message = EventMessage::new(event);
        let event_type = message.event.event_type();
        let charge_point_id = message.event.charge_point_id().map(String::from);

        match self.sender.send(message) {
            Ok(count) => {
                debug!(
                    "Event published: type={}, charge_point={:?}, subscribers={}",
                    event_type, charge_point_id, count
                );
            }
            Err(_) => {
                // No subscribers - this is normal if no UI clients connected
                debug!(
                    "Event published (no subscribers): type={}, charge_point={:?}",
                    event_type, charge_point_id
                );
            }
        }
    }

    /// Subscribe to receive events
    pub fn subscribe(&self) -> EventSubscriber {
        let receiver = self.sender.subscribe();
        self.subscriber_count.fetch_add(1, Ordering::SeqCst);
        let count = self.subscriber_count.load(Ordering::SeqCst);
        info!("New event subscriber, total: {}", count);
        
        EventSubscriber {
            receiver,
            subscriber_count: self.subscriber_count.clone(),
        }
    }

    /// Get current subscriber count
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
    /// Receive the next event
    pub async fn recv(&mut self) -> Option<EventMessage> {
        loop {
            match self.receiver.recv().await {
                Ok(msg) => return Some(msg),
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    warn!("Subscriber lagged, {} events missed", count);
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
        info!("Event subscriber disconnected, remaining: {}", prev - 1);
    }
}

/// Shared event bus type
pub type SharedEventBus = Arc<EventBus>;

/// Create a shared event bus
pub fn create_event_bus() -> SharedEventBus {
    Arc::new(EventBus::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notifications::events::HeartbeatEvent;
    use chrono::Utc;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new();
        let mut subscriber = bus.subscribe();

        let event = Event::HeartbeatReceived(HeartbeatEvent {
            charge_point_id: "CP001".to_string(),
            timestamp: Utc::now(),
        });

        bus.publish(event);

        let received = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            subscriber.recv(),
        )
        .await
        .expect("Timeout")
        .expect("No message");

        assert_eq!(received.event.event_type(), "heartbeat_received");
    }

    #[test]
    fn test_subscriber_count() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);

        let _sub1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _sub2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);

        drop(_sub1);
        assert_eq!(bus.subscriber_count(), 1);
    }
}
