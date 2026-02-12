//! Billing service for calculating and managing charging costs

use std::sync::Arc;

use tracing::info;

use crate::domain::{
    BillingStatus, CostBreakdown, DomainResult, RepositoryProvider, Tariff, TransactionBilling,
};
use crate::shared::errors::DomainError;

/// Service for billing operations
pub struct BillingService {
    repos: Arc<dyn RepositoryProvider>,
}

impl BillingService {
    pub fn new(repos: Arc<dyn RepositoryProvider>) -> Self {
        Self { repos }
    }

    pub async fn calculate_transaction_billing(
        &self,
        transaction_id: i32,
        tariff_id: Option<i32>,
    ) -> DomainResult<TransactionBilling> {
        let transaction =
            self.repos
                .transactions()
                .find_by_id(transaction_id)
                .await?
                .ok_or(DomainError::NotFound {
                    entity: "Transaction",
                    field: "id",
                    value: transaction_id.to_string(),
                })?;

        if transaction.stopped_at.is_none() {
            return Err(DomainError::Validation(
                "Cannot calculate billing for active transaction".to_string(),
            ));
        }

        let tariff = if let Some(id) = tariff_id {
            self.repos
                .tariffs()
                .find_by_id(id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    entity: "Tariff",
                    field: "id",
                    value: id.to_string(),
                })?
        } else {
            self.repos
                .tariffs()
                .find_default()
                .await?
                .ok_or_else(|| DomainError::Validation("No default tariff found".to_string()))?
        };

        let energy_wh = transaction.energy_consumed().unwrap_or(0);
        let duration_seconds = transaction
            .stopped_at
            .map(|stop| (stop - transaction.started_at).num_seconds())
            .unwrap_or(0);

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

        self.repos
            .billing()
            .update_billing(billing.clone())
            .await?;

        info!(
            transaction_id,
            total_cost = billing.total_cost as f64 / 100.0,
            currency = tariff.currency.as_str(),
            energy_wh,
            duration_seconds,
            "Transaction billing calculated"
        );

        Ok(billing)
    }

    pub async fn get_transaction_billing(
        &self,
        transaction_id: i32,
    ) -> DomainResult<Option<TransactionBilling>> {
        self.repos.billing().get_billing(transaction_id).await
    }

    pub async fn update_billing_status(
        &self,
        transaction_id: i32,
        status: BillingStatus,
    ) -> DomainResult<()> {
        let mut billing = self
            .repos
            .billing()
            .get_billing(transaction_id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Billing",
                field: "transaction_id",
                value: transaction_id.to_string(),
            })?;

        billing.status = status.clone();
        self.repos.billing().update_billing(billing).await?;

        info!(transaction_id, ?status, "Billing status updated");

        Ok(())
    }

    pub async fn get_tariff(&self, id: i32) -> DomainResult<Option<Tariff>> {
        self.repos.tariffs().find_by_id(id).await
    }

    pub async fn get_default_tariff(&self) -> DomainResult<Option<Tariff>> {
        self.repos.tariffs().find_default().await
    }

    pub async fn list_tariffs(&self) -> DomainResult<Vec<Tariff>> {
        self.repos.tariffs().find_all().await
    }

    pub async fn create_tariff(&self, tariff: Tariff) -> DomainResult<Tariff> {
        self.repos.tariffs().save(tariff).await
    }

    pub async fn update_tariff(&self, tariff: Tariff) -> DomainResult<()> {
        self.repos.tariffs().update(tariff).await
    }

    pub async fn delete_tariff(&self, id: i32) -> DomainResult<()> {
        self.repos.tariffs().delete(id).await
    }

    pub fn calculate_cost_preview(
        &self,
        tariff: &Tariff,
        energy_wh: i32,
        duration_seconds: i64,
    ) -> CostBreakdown {
        tariff.calculate_cost_breakdown(energy_wh, duration_seconds)
    }
}
