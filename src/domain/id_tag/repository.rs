//! IdTag repository interface

use async_trait::async_trait;

use crate::domain::DomainResult;

#[async_trait]
pub trait IdTagRepository: Send + Sync {
    async fn is_valid(&self, id_tag: &str) -> DomainResult<bool>;
    async fn get_auth_status(&self, id_tag: &str) -> DomainResult<Option<String>>;
    async fn add(&self, id_tag: String) -> DomainResult<()>;
    async fn remove(&self, id_tag: &str) -> DomainResult<()>;
}
