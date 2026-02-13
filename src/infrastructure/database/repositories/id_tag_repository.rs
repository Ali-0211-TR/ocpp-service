//! SeaORM implementation of IdTagRepository

use async_trait::async_trait;
use chrono::Utc;
use log::debug;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use crate::domain::id_tag::IdTagRepository;
use crate::domain::{DomainError, DomainResult};
use crate::infrastructure::database::entities::id_tag;

pub struct SeaOrmIdTagRepository {
    db: DatabaseConnection,
}

impl SeaOrmIdTagRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn db_err(e: sea_orm::DbErr) -> DomainError {
    DomainError::Validation(format!("Database error: {}", e))
}

#[async_trait]
impl IdTagRepository for SeaOrmIdTagRepository {
    async fn is_valid(&self, id_tag_value: &str) -> DomainResult<bool> {
        if id_tag_value.is_empty() {
            return Ok(false);
        }

        let tag = id_tag::Entity::find_by_id(id_tag_value)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        match tag {
            Some(t) => {
                let is_valid = t.is_valid();
                if is_valid {
                    let mut active: id_tag::ActiveModel = t.into();
                    active.last_used_at = Set(Some(Utc::now()));
                    let _ = active.update(&self.db).await;
                }
                Ok(is_valid)
            }
            None => {
                debug!("IdTag '{}' not found in database", id_tag_value);
                Ok(false)
            }
        }
    }

    async fn get_auth_status(&self, id_tag_value: &str) -> DomainResult<Option<String>> {
        if id_tag_value.is_empty() {
            return Ok(None);
        }

        let tag = id_tag::Entity::find_by_id(id_tag_value)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        match tag {
            Some(t) => {
                let status = t.get_auth_status();
                let status_str = match status {
                    id_tag::IdTagStatus::Accepted => "Accepted",
                    id_tag::IdTagStatus::Blocked => "Blocked",
                    id_tag::IdTagStatus::Expired => "Expired",
                    id_tag::IdTagStatus::Invalid => "Invalid",
                    id_tag::IdTagStatus::ConcurrentTx => "ConcurrentTx",
                };

                if status == id_tag::IdTagStatus::Accepted {
                    let mut active: id_tag::ActiveModel = t.into();
                    active.last_used_at = Set(Some(Utc::now()));
                    let _ = active.update(&self.db).await;
                }

                Ok(Some(status_str.to_string()))
            }
            None => Ok(None),
        }
    }

    async fn add(&self, id_tag_value: String) -> DomainResult<()> {
        let now = Utc::now();
        let new_tag = id_tag::ActiveModel {
            id_tag: Set(id_tag_value),
            parent_id_tag: Set(None),
            status: Set(id_tag::IdTagStatus::Accepted),
            user_id: Set(None),
            name: Set(None),
            expiry_date: Set(None),
            max_active_transactions: Set(None),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            last_used_at: Set(None),
        };
        new_tag.insert(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn remove(&self, id_tag_value: &str) -> DomainResult<()> {
        id_tag::Entity::delete_by_id(id_tag_value)
            .exec(&self.db)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn get_parent_id_tag(&self, id_tag_value: &str) -> DomainResult<Option<String>> {
        if id_tag_value.is_empty() {
            return Ok(None);
        }

        let tag = id_tag::Entity::find_by_id(id_tag_value)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        Ok(tag.and_then(|t| t.parent_id_tag))
    }
}
