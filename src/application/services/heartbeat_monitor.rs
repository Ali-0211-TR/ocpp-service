//! Heartbeat Monitor Service
//!
//! Monitors charge point heartbeats and marks stations as offline.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::application::session::SharedSessionRegistry;
use crate::domain::{ChargePointStatus, DomainResult, Storage};
use crate::support::shutdown::ShutdownSignal;

/// Configuration for heartbeat monitoring
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    pub check_interval_secs: u64,
    pub offline_threshold_secs: i64,
    pub unavailable_threshold_secs: i64,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 60,
            offline_threshold_secs: 180,
            unavailable_threshold_secs: 600,
        }
    }
}

/// Information about a charge point's heartbeat status
#[derive(Debug, Clone)]
pub struct HeartbeatStatus {
    pub charge_point_id: String,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
    pub is_connected: bool,
    pub status: ChargePointStatus,
    pub seconds_since_heartbeat: Option<i64>,
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total: usize,
    pub online: usize,
    pub offline: usize,
    pub stale: usize,
}

/// Heartbeat Monitor Service
pub struct HeartbeatMonitor {
    storage: Arc<dyn Storage>,
    session_registry: SharedSessionRegistry,
    config: HeartbeatConfig,
    running: Arc<RwLock<bool>>,
}

impl HeartbeatMonitor {
    pub fn new(storage: Arc<dyn Storage>, session_registry: SharedSessionRegistry) -> Self {
        Self {
            storage,
            session_registry,
            config: HeartbeatConfig::default(),
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_config(mut self, config: HeartbeatConfig) -> Self {
        self.config = config;
        self
    }

    pub fn start(&self, shutdown: ShutdownSignal) {
        let storage = self.storage.clone();
        let session_registry = self.session_registry.clone();
        let config = self.config.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            {
                let mut r = running.write().await;
                *r = true;
            }

            info!(
                check_interval = config.check_interval_secs,
                offline_threshold = config.offline_threshold_secs,
                "💓 Heartbeat monitor started"
            );

            let mut interval =
                tokio::time::interval(Duration::from_secs(config.check_interval_secs));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = check_heartbeats(&storage, &session_registry, &config).await {
                            warn!(error = %e, "Heartbeat check error");
                        }
                    }
                    _ = shutdown.notified().wait() => {
                        info!("💓 Heartbeat monitor shutting down");
                        break;
                    }
                }
            }

            {
                let mut r = running.write().await;
                *r = false;
            }

            info!("💓 Heartbeat monitor stopped");
        });
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub async fn get_all_statuses(&self) -> DomainResult<Vec<HeartbeatStatus>> {
        let charge_points = self.storage.list_charge_points().await?;
        let now = Utc::now();

        let statuses = charge_points
            .into_iter()
            .map(|cp| {
                let is_connected = self.session_registry.is_connected(&cp.id);
                let seconds_since = cp
                    .last_heartbeat
                    .map(|hb| now.signed_duration_since(hb).num_seconds());

                HeartbeatStatus {
                    charge_point_id: cp.id.clone(),
                    last_heartbeat: cp.last_heartbeat,
                    last_seen: cp.last_heartbeat,
                    is_connected,
                    status: cp.status,
                    seconds_since_heartbeat: seconds_since,
                }
            })
            .collect();

        Ok(statuses)
    }

    pub async fn get_status(&self, charge_point_id: &str) -> DomainResult<Option<HeartbeatStatus>> {
        let cp = self.storage.get_charge_point(charge_point_id).await?;
        let now = Utc::now();

        Ok(cp.map(|cp| {
            let is_connected = self.session_registry.is_connected(&cp.id);
            let seconds_since = cp
                .last_heartbeat
                .map(|hb| now.signed_duration_since(hb).num_seconds());

            HeartbeatStatus {
                charge_point_id: cp.id.clone(),
                last_heartbeat: cp.last_heartbeat,
                last_seen: cp.last_heartbeat,
                is_connected,
                status: cp.status,
                seconds_since_heartbeat: seconds_since,
            }
        }))
    }

    pub async fn get_online_charge_points(&self) -> Vec<String> {
        self.session_registry.connected_ids()
    }

    pub async fn get_connection_stats(&self) -> DomainResult<ConnectionStats> {
        let charge_points = self.storage.list_charge_points().await?;
        let total = charge_points.len();

        let online = charge_points
            .iter()
            .filter(|cp| self.session_registry.is_connected(&cp.id))
            .count();

        let offline = total - online;

        let stale = charge_points
            .iter()
            .filter(|cp| {
                if let Some(hb) = cp.last_heartbeat {
                    let elapsed = Utc::now().signed_duration_since(hb).num_seconds();
                    elapsed > self.config.offline_threshold_secs
                } else {
                    true
                }
            })
            .count();

        Ok(ConnectionStats {
            total,
            online,
            offline,
            stale,
        })
    }
}

async fn check_heartbeats(
    storage: &Arc<dyn Storage>,
    session_registry: &SharedSessionRegistry,
    config: &HeartbeatConfig,
) -> DomainResult<()> {
    let charge_points = storage.list_charge_points().await?;
    let now = Utc::now();

    debug!(count = charge_points.len(), "Checking heartbeats");

    for cp in charge_points {
        let is_connected = session_registry.is_connected(&cp.id);
        let current_status = cp.status.clone();

        let new_status = if is_connected {
            if let Some(last_hb) = cp.last_heartbeat {
                let elapsed = now.signed_duration_since(last_hb).num_seconds();
                if elapsed > config.unavailable_threshold_secs {
                    ChargePointStatus::Unavailable
                } else {
                    ChargePointStatus::Online
                }
            } else {
                ChargePointStatus::Online
            }
        } else if let Some(last_hb) = cp.last_heartbeat {
            let elapsed = now.signed_duration_since(last_hb).num_seconds();
            if elapsed > config.unavailable_threshold_secs {
                ChargePointStatus::Unavailable
            } else {
                ChargePointStatus::Offline
            }
        } else {
            ChargePointStatus::Unknown
        };

        if new_status != current_status {
            info!(
                charge_point_id = cp.id.as_str(),
                ?current_status,
                ?new_status,
                is_connected,
                "💓 Status changed"
            );

            if let Err(e) = storage
                .update_charge_point_status(&cp.id, new_status.clone())
                .await
            {
                warn!(
                    charge_point_id = cp.id.as_str(),
                    error = %e,
                    "Failed to update status"
                );
            }
        }
    }

    Ok(())
}
