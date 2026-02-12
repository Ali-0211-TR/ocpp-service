use async_trait::async_trait;

use super::{CreateUserDto, GetUserDto, UpdateUserDto, User};
use crate::domain::DomainResult;
use crate::shared::PaginatedResult;

#[async_trait]
pub trait UserRepositoryInterface: Send + Sync {
    async fn create_user(&self, dto: CreateUserDto) -> DomainResult<()>;

    async fn list_users(&self, dto: GetUserDto) -> DomainResult<PaginatedResult<User>>;
    async fn get_user_by_username(&self, username: &str) -> DomainResult<Option<User>>;
    async fn get_user_by_email(&self, email: &str) -> DomainResult<Option<User>>;
    async fn get_user_by_id(&self, id: &str) -> DomainResult<Option<User>>;

    async fn update_user(&self, id: &str, dto: UpdateUserDto) -> DomainResult<Option<User>>;
    async fn update_user_password(&self, id: &str, new_password_hash: &str) -> DomainResult<()>;
    async fn delete_user(&self, id: &str) -> DomainResult<()>;
}
