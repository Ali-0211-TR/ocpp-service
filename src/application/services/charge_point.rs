//! Charge point business logic service

use std::sync::Arc;

use dashmap::DashMap;
use log::info;

use crate::domain::{ChargePoint, ChargingLimitType, ConnectorStatus, DomainResult, Transaction};
use crate::infrastructure::Storage;

/// Pending charging limit (set via remote start, applied when transaction starts)
#[derive(Debug, Clone)]
pub struct PendingChargingLimit {
    pub limit_type: ChargingLimitType,
    pub limit_value: f64,
}

/// Service for charge point business operations
pub struct ChargePointService {
    storage: Arc<dyn Storage>,
    /// Pending charging limits per (charge_point_id, connector_id)
    pending_limits: DashMap<(String, u32), PendingChargingLimit>,
}

impl ChargePointService {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            storage,
            pending_limits: DashMap::new(),
        }
    }

    /// Store a pending charging limit for a connector
    pub fn set_pending_limit(
        &self,
        charge_point_id: &str,
        connector_id: u32,
        limit_type: ChargingLimitType,
        limit_value: f64,
    ) {
        self.pending_limits.insert(
            (charge_point_id.to_string(), connector_id),
            PendingChargingLimit { limit_type, limit_value },
        );
    }

    /// Take and remove a pending charging limit for a connector
    fn take_pending_limit(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> Option<PendingChargingLimit> {
        self.pending_limits
            .remove(&(charge_point_id.to_string(), connector_id))
            .map(|(_, v)| v)
    }

    /// Register a new charge point or update existing one
    pub async fn register_or_update(
        &self,
        charge_point_id: &str,
        vendor: &str,
        model: &str,
        serial_number: Option<&str>,
        firmware_version: Option<&str>,
    ) -> DomainResult<ChargePoint> {
        let existing = self.storage.get_charge_point(charge_point_id).await?;

        let mut cp = existing.unwrap_or_else(|| ChargePoint::new(charge_point_id));

        cp.vendor = Some(vendor.to_string());
        cp.model = Some(model.to_string());
        cp.serial_number = serial_number.map(String::from);
        cp.firmware_version = firmware_version.map(String::from);
        cp.set_online(); // Set status to Online when registering
        cp.update_heartbeat();

        // Save or update
        if self.storage.get_charge_point(charge_point_id).await?.is_some() {
            self.storage.update_charge_point(cp.clone()).await?;
        } else {
            self.storage.save_charge_point(cp.clone()).await?;
        }

        info!(
            "Charge point registered: {} ({} {})",
            charge_point_id, vendor, model
        );

        Ok(cp)
    }

    /// Update heartbeat for a charge point
    pub async fn heartbeat(&self, charge_point_id: &str) -> DomainResult<()> {
        if let Some(mut cp) = self.storage.get_charge_point(charge_point_id).await? {
            cp.update_heartbeat();
            self.storage.update_charge_point(cp).await?;
        }
        Ok(())
    }

    /// Update connector status
    pub async fn update_connector_status(
        &self,
        charge_point_id: &str,
        connector_id: u32,
        status: ConnectorStatus,
    ) -> DomainResult<()> {
        if let Some(mut cp) = self.storage.get_charge_point(charge_point_id).await? {
            cp.update_connector_status(connector_id, status);
            self.storage.update_charge_point(cp).await?;
        }
        Ok(())
    }

    /// Ensure N connectors exist on a charge point (auto-provisioning).
    /// Creates connector 0 (station) plus connectors 1..=num_connectors.
    pub async fn ensure_connectors(
        &self,
        charge_point_id: &str,
        num_connectors: u32,
    ) -> DomainResult<()> {
        if let Some(mut cp) = self.storage.get_charge_point(charge_point_id).await? {
            cp.ensure_connectors(num_connectors);
            self.storage.update_charge_point(cp).await?;
            info!(
                "[{}] Ensured {} connectors exist (0..={})",
                charge_point_id, num_connectors + 1, num_connectors
            );
        }
        Ok(())
    }

    /// Add a single connector to a charge point.
    pub async fn add_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> DomainResult<bool> {
        if let Some(mut cp) = self.storage.get_charge_point(charge_point_id).await? {
            let added = cp.add_connector(connector_id);
            if added {
                self.storage.update_charge_point(cp).await?;
                info!("[{}] Connector {} added", charge_point_id, connector_id);
            }
            Ok(added)
        } else {
            Err(crate::domain::DomainError::ChargePointNotFound(
                charge_point_id.to_string(),
            ))
        }
    }

    /// Remove a connector from a charge point.
    pub async fn remove_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> DomainResult<bool> {
        if let Some(mut cp) = self.storage.get_charge_point(charge_point_id).await? {
            let removed = cp.remove_connector(connector_id);
            if removed {
                self.storage.update_charge_point(cp).await?;
                info!("[{}] Connector {} removed", charge_point_id, connector_id);
            }
            Ok(removed)
        } else {
            Err(crate::domain::DomainError::ChargePointNotFound(
                charge_point_id.to_string(),
            ))
        }
    }

    /// Authorize an ID tag
    pub async fn authorize(&self, id_tag: &str) -> DomainResult<bool> {
        self.storage.is_id_tag_valid(id_tag).await
    }
    
    /// Get authorization status for an ID tag
    /// Returns the OCPP status string (Accepted, Blocked, Expired, Invalid, ConcurrentTx)
    /// Returns None if the tag is not found (should be treated as Invalid)
    pub async fn get_auth_status(&self, id_tag: &str) -> DomainResult<Option<String>> {
        self.storage.get_id_tag_auth_status(id_tag).await
    }

    /// Start a new transaction
    pub async fn start_transaction(
        &self,
        charge_point_id: &str,
        connector_id: u32,
        id_tag: &str,
        meter_start: i32,
    ) -> DomainResult<Transaction> {
        let transaction_id = self.storage.next_transaction_id().await;

        let mut transaction = Transaction::new(
            transaction_id,
            charge_point_id,
            connector_id,
            id_tag,
            meter_start,
        );

        // Apply pending charging limits if any
        if let Some(limit) = self.take_pending_limit(charge_point_id, connector_id) {
            info!(
                "Applying pending charging limit to transaction {}: {:?} = {}",
                transaction_id, limit.limit_type, limit.limit_value
            );
            transaction.limit_type = Some(limit.limit_type);
            transaction.limit_value = Some(limit.limit_value);
        }

        self.storage.save_transaction(transaction.clone()).await?;

        info!(
            "Transaction {} started: CP={}, Connector={}, IdTag={}, Limits={:?}/{:?}",
            transaction_id, charge_point_id, connector_id, id_tag,
            transaction.limit_type, transaction.limit_value
        );

        Ok(transaction)
    }

    /// Stop a transaction
    pub async fn stop_transaction(
        &self,
        transaction_id: i32,
        meter_stop: i32,
        reason: Option<String>,
    ) -> DomainResult<Transaction> {
        let mut transaction = self
            .storage
            .get_transaction(transaction_id)
            .await?
            .ok_or(crate::domain::DomainError::TransactionNotFound(transaction_id))?;

        transaction.stop(meter_stop, reason);
        self.storage.update_transaction(transaction.clone()).await?;

        if let Some(energy) = transaction.energy_consumed() {
            info!(
                "Transaction {} stopped: Energy consumed = {} Wh",
                transaction_id, energy
            );
        }

        Ok(transaction)
    }

    /// Update live meter data for an active transaction
    pub async fn update_transaction_meter_data(
        &self,
        transaction_id: i32,
        meter_value: Option<i32>,
        power_w: Option<f64>,
        soc: Option<i32>,
    ) -> DomainResult<Option<Transaction>> {
        self.storage
            .update_transaction_meter_data(transaction_id, meter_value, power_w, soc)
            .await?;

        // Return updated transaction for limit checking
        self.storage.get_transaction(transaction_id).await
    }

    /// Get active transaction for a connector
    pub async fn get_active_transaction_for_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> DomainResult<Option<Transaction>> {
        self.storage
            .get_active_transaction_for_connector(charge_point_id, connector_id)
            .await
    }

    /// Get charge point by ID
    pub async fn get_charge_point(&self, id: &str) -> DomainResult<Option<ChargePoint>> {
        self.storage.get_charge_point(id).await
    }

    /// List all charge points
    pub async fn list_charge_points(&self) -> DomainResult<Vec<ChargePoint>> {
        self.storage.list_charge_points().await
    }
}
