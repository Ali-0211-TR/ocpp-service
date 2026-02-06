//! In-memory storage implementation

use std::sync::atomic::{AtomicI32, Ordering};

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;

use super::Storage;
use crate::domain::{
    ChargePoint, DomainError, DomainResult, Tariff, TariffType, Transaction,
    TransactionBilling, TransactionStatus,
};

/// In-memory storage for development and testing
pub struct InMemoryStorage {
    charge_points: DashMap<String, ChargePoint>,
    transactions: DashMap<i32, Transaction>,
    transaction_billing: DashMap<i32, TransactionBilling>,
    tariffs: DashMap<i32, Tariff>,
    valid_id_tags: DashMap<String, ()>,
    transaction_counter: AtomicI32,
    tariff_counter: AtomicI32,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        let storage = Self {
            charge_points: DashMap::new(),
            transactions: DashMap::new(),
            transaction_billing: DashMap::new(),
            tariffs: DashMap::new(),
            valid_id_tags: DashMap::new(),
            transaction_counter: AtomicI32::new(1),
            tariff_counter: AtomicI32::new(1),
        };

        // Add some default valid ID tags for testing
        storage.valid_id_tags.insert("TEST001".to_string(), ());
        storage.valid_id_tags.insert("TEST002".to_string(), ());
        storage.valid_id_tags.insert("ADMIN".to_string(), ());

        // Add default tariff
        let default_tariff = Tariff {
            id: 1,
            name: "Standard".to_string(),
            description: Some("Default tariff".to_string()),
            tariff_type: TariffType::PerKwh,
            price_per_kwh: 250,
            price_per_minute: 0,
            session_fee: 0,
            currency: "UZS".to_string(),
            min_fee: 100,
            max_fee: 0,
            is_active: true,
            is_default: true,
            valid_from: None,
            valid_until: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        storage.tariffs.insert(1, default_tariff);
        storage.tariff_counter.store(2, Ordering::SeqCst);

        storage
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn save_charge_point(&self, charge_point: ChargePoint) -> DomainResult<()> {
        if self.charge_points.contains_key(&charge_point.id) {
            return Err(DomainError::ChargePointAlreadyExists(charge_point.id));
        }
        self.charge_points.insert(charge_point.id.clone(), charge_point);
        Ok(())
    }

    async fn get_charge_point(&self, id: &str) -> DomainResult<Option<ChargePoint>> {
        Ok(self.charge_points.get(id).map(|cp| cp.clone()))
    }

    async fn update_charge_point(&self, charge_point: ChargePoint) -> DomainResult<()> {
        if !self.charge_points.contains_key(&charge_point.id) {
            return Err(DomainError::ChargePointNotFound(charge_point.id));
        }
        self.charge_points.insert(charge_point.id.clone(), charge_point);
        Ok(())
    }

    async fn delete_charge_point(&self, id: &str) -> DomainResult<()> {
        self.charge_points
            .remove(id)
            .ok_or_else(|| DomainError::ChargePointNotFound(id.to_string()))?;
        Ok(())
    }

    async fn list_charge_points(&self) -> DomainResult<Vec<ChargePoint>> {
        Ok(self.charge_points.iter().map(|e| e.value().clone()).collect())
    }

    async fn update_charge_point_status(&self, id: &str, status: crate::domain::ChargePointStatus) -> DomainResult<()> {
        if let Some(mut cp) = self.charge_points.get_mut(id) {
            cp.status = status;
            Ok(())
        } else {
            Err(DomainError::ChargePointNotFound(id.to_string()))
        }
    }

    async fn save_transaction(&self, transaction: Transaction) -> DomainResult<()> {
        self.transactions.insert(transaction.id, transaction);
        Ok(())
    }

    async fn get_transaction(&self, id: i32) -> DomainResult<Option<Transaction>> {
        Ok(self.transactions.get(&id).map(|t| t.clone()))
    }

    async fn update_transaction(&self, transaction: Transaction) -> DomainResult<()> {
        if !self.transactions.contains_key(&transaction.id) {
            return Err(DomainError::TransactionNotFound(transaction.id));
        }
        self.transactions.insert(transaction.id, transaction);
        Ok(())
    }

    async fn get_active_transaction_for_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> DomainResult<Option<Transaction>> {
        Ok(self
            .transactions
            .iter()
            .find(|t| {
                t.charge_point_id == charge_point_id
                    && t.connector_id == connector_id
                    && t.status == TransactionStatus::Active
            })
            .map(|t| t.clone()))
    }

    async fn list_transactions_for_charge_point(
        &self,
        charge_point_id: &str,
    ) -> DomainResult<Vec<Transaction>> {
        Ok(self
            .transactions
            .iter()
            .filter(|t| t.charge_point_id == charge_point_id)
            .map(|t| t.clone())
            .collect())
    }

    async fn list_all_transactions(&self) -> DomainResult<Vec<Transaction>> {
        Ok(self.transactions.iter().map(|t| t.clone()).collect())
    }

    async fn is_id_tag_valid(&self, id_tag: &str) -> DomainResult<bool> {
        // For development: accept any non-empty ID tag
        // In production, check against database
        Ok(!id_tag.is_empty() && (self.valid_id_tags.contains_key(id_tag) || id_tag.starts_with("TEST")))
    }
    
    async fn get_id_tag_auth_status(&self, id_tag: &str) -> DomainResult<Option<String>> {
        // For memory storage, just return Accepted if valid, None if not found
        if !id_tag.is_empty() && (self.valid_id_tags.contains_key(id_tag) || id_tag.starts_with("TEST")) {
            Ok(Some("Accepted".to_string()))
        } else {
            Ok(None)
        }
    }

    async fn add_id_tag(&self, id_tag: String) -> DomainResult<()> {
        self.valid_id_tags.insert(id_tag, ());
        Ok(())
    }

    async fn remove_id_tag(&self, id_tag: &str) -> DomainResult<()> {
        self.valid_id_tags.remove(id_tag);
        Ok(())
    }

    async fn get_tariff(&self, id: i32) -> DomainResult<Option<Tariff>> {
        Ok(self.tariffs.get(&id).map(|t| t.clone()))
    }

    async fn get_default_tariff(&self) -> DomainResult<Option<Tariff>> {
        Ok(self.tariffs.iter().find(|t| t.is_default && t.is_active).map(|t| t.clone()))
    }

    async fn list_tariffs(&self) -> DomainResult<Vec<Tariff>> {
        Ok(self.tariffs.iter().map(|t| t.clone()).collect())
    }

    async fn save_tariff(&self, mut tariff: Tariff) -> DomainResult<Tariff> {
        let id = self.tariff_counter.fetch_add(1, Ordering::SeqCst);
        tariff.id = id;
        tariff.created_at = Utc::now();
        tariff.updated_at = Utc::now();
        self.tariffs.insert(id, tariff.clone());
        Ok(tariff)
    }

    async fn update_tariff(&self, tariff: Tariff) -> DomainResult<()> {
        if !self.tariffs.contains_key(&tariff.id) {
            return Err(DomainError::StorageError(format!("Tariff {} not found", tariff.id)));
        }
        self.tariffs.insert(tariff.id, tariff);
        Ok(())
    }

    async fn delete_tariff(&self, id: i32) -> DomainResult<()> {
        self.tariffs.remove(&id)
            .ok_or_else(|| DomainError::StorageError(format!("Tariff {} not found", id)))?;
        Ok(())
    }

    async fn update_transaction_billing(&self, billing: TransactionBilling) -> DomainResult<()> {
        if !self.transactions.contains_key(&billing.transaction_id) {
            return Err(DomainError::TransactionNotFound(billing.transaction_id));
        }
        self.transaction_billing.insert(billing.transaction_id, billing);
        Ok(())
    }

    async fn get_transaction_billing(&self, transaction_id: i32) -> DomainResult<Option<TransactionBilling>> {
        Ok(self.transaction_billing.get(&transaction_id).map(|b| b.clone()))
    }

    async fn next_transaction_id(&self) -> i32 {
        self.transaction_counter.fetch_add(1, Ordering::SeqCst)
    }
}
