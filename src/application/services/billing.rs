//! Billing service for calculating and managing charging costs

use std::sync::Arc;

use log::info;

use crate::domain::{
    BillingStatus, CostBreakdown, DomainError, DomainResult, Tariff, TransactionBilling,
};
use crate::infrastructure::Storage;

/// Service for billing operations
pub struct BillingService {
    storage: Arc<dyn Storage>,
}

impl BillingService {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Calculate billing for a completed transaction
    /// This should be called after a transaction is stopped
    pub async fn calculate_transaction_billing(
        &self,
        transaction_id: i32,
        tariff_id: Option<i32>,
    ) -> DomainResult<TransactionBilling> {
        // Get the transaction
        let transaction = self
            .storage
            .get_transaction(transaction_id)
            .await?
            .ok_or(DomainError::TransactionNotFound(transaction_id))?;

        // Transaction must be completed
        if transaction.stopped_at.is_none() {
            return Err(DomainError::StorageError(
                "Cannot calculate billing for active transaction".to_string(),
            ));
        }

        // Get the tariff (use specified or default)
        let tariff = if let Some(id) = tariff_id {
            self.storage
                .get_tariff(id)
                .await?
                .ok_or_else(|| DomainError::StorageError(format!("Tariff {} not found", id)))?
        } else {
            self.storage
                .get_default_tariff()
                .await?
                .ok_or_else(|| DomainError::StorageError("No default tariff found".to_string()))?
        };

        // Calculate energy and duration
        let energy_wh = transaction.energy_consumed().unwrap_or(0);
        let duration_seconds = transaction
            .stopped_at
            .map(|stop| (stop - transaction.started_at).num_seconds())
            .unwrap_or(0);

        // Calculate cost breakdown
        let breakdown = tariff.calculate_cost_breakdown(energy_wh, duration_seconds);

        let billing = TransactionBilling {
            transaction_id,
            tariff_id: Some(tariff.id),
            energy_wh,
            duration_seconds,
            energy_cost: breakdown.energy_cost,
            time_cost: breakdown.time_cost,
            session_fee: breakdown.session_fee,
            total_cost: breakdown.total,
            currency: breakdown.currency,
            status: BillingStatus::Calculated,
        };

        // Save billing to database
        self.storage.update_transaction_billing(billing.clone()).await?;

        info!(
            "Transaction {} billing calculated: {} {} (energy: {} Wh, duration: {} s)",
            transaction_id,
            billing.total_cost as f64 / 100.0,
            tariff.currency,
            energy_wh,
            duration_seconds
        );

        Ok(billing)
    }

    /// Get billing details for a transaction
    pub async fn get_transaction_billing(
        &self,
        transaction_id: i32,
    ) -> DomainResult<Option<TransactionBilling>> {
        self.storage.get_transaction_billing(transaction_id).await
    }

    /// Update billing status (e.g., mark as invoiced or paid)
    pub async fn update_billing_status(
        &self,
        transaction_id: i32,
        status: BillingStatus,
    ) -> DomainResult<()> {
        let mut billing = self
            .storage
            .get_transaction_billing(transaction_id)
            .await?
            .ok_or_else(|| {
                DomainError::StorageError(format!(
                    "Billing for transaction {} not found",
                    transaction_id
                ))
            })?;

        billing.status = status.clone();
        self.storage.update_transaction_billing(billing).await?;

        info!("Transaction {} billing status updated to {:?}", transaction_id, status);

        Ok(())
    }

    /// Get tariff by ID
    pub async fn get_tariff(&self, id: i32) -> DomainResult<Option<Tariff>> {
        self.storage.get_tariff(id).await
    }

    /// Get default tariff
    pub async fn get_default_tariff(&self) -> DomainResult<Option<Tariff>> {
        self.storage.get_default_tariff().await
    }

    /// List all tariffs
    pub async fn list_tariffs(&self) -> DomainResult<Vec<Tariff>> {
        self.storage.list_tariffs().await
    }

    /// Create a new tariff
    pub async fn create_tariff(&self, tariff: Tariff) -> DomainResult<Tariff> {
        self.storage.save_tariff(tariff).await
    }

    /// Update a tariff
    pub async fn update_tariff(&self, tariff: Tariff) -> DomainResult<()> {
        self.storage.update_tariff(tariff).await
    }

    /// Delete a tariff
    pub async fn delete_tariff(&self, id: i32) -> DomainResult<()> {
        self.storage.delete_tariff(id).await
    }

    /// Calculate cost preview (without saving)
    pub fn calculate_cost_preview(
        &self,
        tariff: &Tariff,
        energy_wh: i32,
        duration_seconds: i64,
    ) -> CostBreakdown {
        tariff.calculate_cost_breakdown(energy_wh, duration_seconds)
    }
}
