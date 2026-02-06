//! Heartbeat Monitor Service
//!
//! Monitors charge point heartbeats and marks stations as offline
//! when they stop responding.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use tokio::sync::RwLock;

use crate::domain::{ChargePointStatus, DomainResult};
use crate::infrastructure::Storage;
use crate::infrastructure::server::ShutdownSignal;
use crate::session::SharedSessionManager;

/// Configuration for heartbeat monitoring
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// How often to check for stale connections (in seconds)
    pub check_interval_secs: u64,
    /// How long before a station is considered offline (in seconds)
    pub offline_threshold_secs: i64,
    /// How long before a station is considered unavailable (in seconds)
    pub unavailable_threshold_secs: i64,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 60,        // Check every minute
            offline_threshold_secs: 180,     // 3 minutes without heartbeat = offline
            unavailable_threshold_secs: 600, // 10 minutes = unavailable
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

/// Heartbeat Monitor Service
///
/// Runs in the background and monitors charge point heartbeats,
/// updating their status when they go offline.
pub struct HeartbeatMonitor {
    storage: Arc<dyn Storage>,
    session_manager: SharedSessionManager,
    config: HeartbeatConfig,
    /// Running state
    running: Arc<RwLock<bool>>,
}

impl HeartbeatMonitor {
    pub fn new(
        storage: Arc<dyn Storage>,
        session_manager: SharedSessionManager,
    ) -> Self {
        Self {
            storage,
            session_manager,
            config: HeartbeatConfig::default(),
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_config(mut self, config: HeartbeatConfig) -> Self {
        self.config = config;
        self
    }

    /// Start the heartbeat monitor background task
    pub fn start(&self, shutdown: ShutdownSignal) {
        let storage = self.storage.clone();
        let session_manager = self.session_manager.clone();
        let config = self.config.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            {
                let mut r = running.write().await;
                *r = true;
            }

            info!("💓 Heartbeat monitor started (check interval: {}s, offline threshold: {}s)",
                config.check_interval_secs, config.offline_threshold_secs);

            let mut interval = tokio::time::interval(Duration::from_secs(config.check_interval_secs));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = check_heartbeats(&storage, &session_manager, &config).await {
                            warn!("Heartbeat check error: {}", e);
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

    /// Check if the monitor is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get heartbeat status for all charge points
    pub async fn get_all_statuses(&self) -> DomainResult<Vec<HeartbeatStatus>> {
        let charge_points = self.storage.list_charge_points().await?;
        let now = Utc::now();

        let statuses: Vec<HeartbeatStatus> = charge_points
            .into_iter()
            .map(|cp| {
                let is_connected = self.session_manager.is_connected(&cp.id);
                let seconds_since = cp.last_heartbeat.map(|hb| {
                    now.signed_duration_since(hb).num_seconds()
                });

                HeartbeatStatus {
                    charge_point_id: cp.id.clone(),
                    last_heartbeat: cp.last_heartbeat,
                    last_seen: cp.last_heartbeat, // Use last_heartbeat as last_seen
                    is_connected,
                    status: cp.status,
                    seconds_since_heartbeat: seconds_since,
                }
            })
            .collect();

        Ok(statuses)
    }

    /// Get heartbeat status for a specific charge point
    pub async fn get_status(&self, charge_point_id: &str) -> DomainResult<Option<HeartbeatStatus>> {
        let cp = self.storage.get_charge_point(charge_point_id).await?;
        let now = Utc::now();

        Ok(cp.map(|cp| {
            let is_connected = self.session_manager.is_connected(&cp.id);
            let seconds_since = cp.last_heartbeat.map(|hb| {
                now.signed_duration_since(hb).num_seconds()
            });

            HeartbeatStatus {
                charge_point_id: cp.id.clone(),
                last_heartbeat: cp.last_heartbeat,
                last_seen: cp.last_heartbeat, // Use last_heartbeat as last_seen
                is_connected,
                status: cp.status,
                seconds_since_heartbeat: seconds_since,
            }
        }))
    }

    /// Get list of online charge points
    pub async fn get_online_charge_points(&self) -> Vec<String> {
        self.session_manager.connected_ids()
    }

    /// Get count of online vs offline charge points
    pub async fn get_connection_stats(&self) -> DomainResult<ConnectionStats> {
        let charge_points = self.storage.list_charge_points().await?;
        let total = charge_points.len();
        
        let online = charge_points
            .iter()
            .filter(|cp| self.session_manager.is_connected(&cp.id))
            .count();

        let offline = total - online;

        let stale = charge_points
            .iter()
            .filter(|cp| {
                if let Some(hb) = cp.last_heartbeat {
                    let elapsed = Utc::now().signed_duration_since(hb).num_seconds();
                    elapsed > self.config.offline_threshold_secs
                } else {
                    true // No heartbeat = considered stale
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

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total: usize,
    pub online: usize,
    pub offline: usize,
    pub stale: usize,
}

/// Check all charge points for heartbeat timeouts
async fn check_heartbeats(
    storage: &Arc<dyn Storage>,
    session_manager: &SharedSessionManager,
    config: &HeartbeatConfig,
) -> DomainResult<()> {
    let charge_points = storage.list_charge_points().await?;
    let now = Utc::now();

    debug!("Checking heartbeats for {} charge points", charge_points.len());

    for cp in charge_points {
        let is_connected = session_manager.is_connected(&cp.id);
        let current_status = cp.status.clone();

        // Determine new status based on heartbeat and connection
        let new_status = if is_connected {
            // Connected via WebSocket
            if let Some(last_hb) = cp.last_heartbeat {
                let elapsed = now.signed_duration_since(last_hb).num_seconds();
                
                if elapsed > config.unavailable_threshold_secs {
                    // Connected but no heartbeat for too long - might be stuck
                    ChargePointStatus::Unavailable
                } else if elapsed > config.offline_threshold_secs {
                    // Connected but heartbeat delayed
                    ChargePointStatus::Online // Still consider online if WebSocket is active
                } else {
                    ChargePointStatus::Online
                }
            } else {
                // Connected but never sent heartbeat yet
                ChargePointStatus::Online
            }
        } else {
            // Not connected via WebSocket
            if let Some(last_hb) = cp.last_heartbeat {
                let elapsed = now.signed_duration_since(last_hb).num_seconds();
                
                if elapsed > config.unavailable_threshold_secs {
                    ChargePointStatus::Unavailable
                } else {
                    ChargePointStatus::Offline
                }
            } else {
                // Never connected
                ChargePointStatus::Unknown
            }
        };

        // Update status if changed
        if new_status != current_status {
            info!(
                "💓 [{}] Status changed: {:?} → {:?} (connected: {}, last_hb: {:?})",
                cp.id, current_status, new_status, is_connected,
                cp.last_heartbeat.map(|hb| now.signed_duration_since(hb).num_seconds())
            );

            // Update in database
            if let Err(e) = storage.update_charge_point_status(&cp.id, new_status.clone()).await {
                warn!("Failed to update status for {}: {}", cp.id, e);
            }
        }
    }

    Ok(())
}
