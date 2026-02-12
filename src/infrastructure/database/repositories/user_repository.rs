use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

use crate::domain::{
    CreateUserDto, GetUserDto, UpdateUserDto, UserRepositoryInterface,
    DomainError, DomainResult, User, UserRole,
};
use crate::infrastructure::database::entities::user;
use crate::shared::PaginatedResult;

pub struct UserRepository {
    db: DatabaseConnection,
}

impl UserRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Conversion helpers ──────────────────────────────────────────

fn entity_role_to_domain(role: user::UserRole) -> UserRole {
    match role {
        user::UserRole::Admin => UserRole::Admin,
        user::UserRole::Operator => UserRole::Operator,
        user::UserRole::Viewer => UserRole::Viewer,
    }
}

fn domain_role_to_entity(role: &UserRole) -> user::UserRole {
    match role {
        UserRole::Admin => user::UserRole::Admin,
        UserRole::Operator => user::UserRole::Operator,
        UserRole::Viewer => user::UserRole::Viewer,
    }
}

fn user_model_to_domain(model: user::Model) -> User {
    User {
        id: model.id,
        username: model.username,
        email: model.email,
        password_hash: model.password_hash,
        role: entity_role_to_domain(model.role),
        is_active: model.is_active,
        created_at: model.created_at,
        updated_at: model.updated_at,
        last_login_at: model.last_login_at,
    }
}

fn db_err(e: sea_orm::DbErr) -> DomainError {
    DomainError::Validation(format!("Database error: {}", e))
}

// ── Repository implementation ───────────────────────────────────

#[async_trait]
impl UserRepositoryInterface for UserRepository {
    async fn create_user(&self, dto: CreateUserDto) -> DomainResult<()> {
        use crate::infrastructure::crypto::password::hash_password;

        let now = Utc::now();
        let id = uuid::Uuid::new_v4().to_string();

        let password_hash = hash_password(&dto.password)
            .map_err(|e| DomainError::Validation(format!("Failed to hash password: {}", e)))?;

        let role = dto
            .role
            .as_ref()
            .map_or(user::UserRole::Viewer, |r| domain_role_to_entity(r));

        let new_user = user::ActiveModel {
            id: Set(id),
            username: Set(dto.username),
            email: Set(dto.email),
            password_hash: Set(password_hash),
            role: Set(role),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            last_login_at: Set(None),
        };

        new_user.insert(&self.db).await.map_err(|e| {
            if e.to_string().contains("UNIQUE") || e.to_string().contains("duplicate") {
                DomainError::Conflict("Username or email already exists".to_string())
            } else {
                db_err(e)
            }
        })?;

        Ok(())
    }

    async fn list_users(&self, dto: GetUserDto) -> DomainResult<PaginatedResult<User>> {
        let page = dto.page.unwrap_or(1).max(1);
        let page_size = dto.page_size.unwrap_or(20).min(100).max(1);

        let mut query = user::Entity::find();

        // Apply search filter (username or email)
        if let Some(ref search) = dto.search {
            query = query.filter(
                user::Column::Username
                    .contains(search)
                    .or(user::Column::Email.contains(search)),
            );
        }

        // Apply role filter
        if let Some(ref role) = dto.role {
            query = query.filter(user::Column::Role.eq(domain_role_to_entity(role)));
        }

        // Apply sorting
        match dto.sort_by.as_deref() {
            Some("username") => {
                query = query.order_by_asc(user::Column::Username);
            }
            Some("email") => {
                query = query.order_by_asc(user::Column::Email);
            }
            Some("role") => {
                query = query.order_by_asc(user::Column::Role);
            }
            _ => {
                query = query.order_by_desc(user::Column::CreatedAt);
            }
        }

        // Count total
        let total = query.clone().count(&self.db).await.map_err(db_err)?;

        // Paginate
        let offset = ((page - 1) * page_size) as u64;
        let models = query
            .offset(offset)
            .limit(page_size as u64)
            .all(&self.db)
            .await
            .map_err(db_err)?;

        let items: Vec<User> = models.into_iter().map(user_model_to_domain).collect();

        Ok(PaginatedResult::new(items, total, page, page_size))
    }

    async fn get_user_by_username(&self, username: &str) -> DomainResult<Option<User>> {
        let model = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(db_err)?;

        Ok(model.map(user_model_to_domain))
    }

    async fn get_user_by_email(&self, email: &str) -> DomainResult<Option<User>> {
        let model = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(db_err)?;

        Ok(model.map(user_model_to_domain))
    }

    async fn get_user_by_id(&self, id: &str) -> DomainResult<Option<User>> {
        let model = user::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        Ok(model.map(user_model_to_domain))
    }

    async fn update_user(&self, id: &str, dto: UpdateUserDto) -> DomainResult<Option<User>> {
        let existing = user::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        let Some(existing) = existing else {
            return Ok(None);
        };

        let mut active: user::ActiveModel = existing.into();

        if let Some(username) = dto.username {
            active.username = Set(username);
        }
        if let Some(email) = dto.email {
            active.email = Set(email);
        }

        active.updated_at = Set(Utc::now());

        let updated = active.update(&self.db).await.map_err(|e| {
            if e.to_string().contains("UNIQUE") || e.to_string().contains("duplicate") {
                DomainError::Conflict("Username or email already exists".to_string())
            } else {
                db_err(e)
            }
        })?;

        Ok(Some(user_model_to_domain(updated)))
    }

    async fn update_user_password(&self, id: &str, new_password_hash: &str) -> DomainResult<()> {
        let existing = user::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        let Some(existing) = existing else {
            return Err(DomainError::NotFound {
                entity: "User",
                field: "id",
                value: id.to_string(),
            });
        };

        let mut active: user::ActiveModel = existing.into();
        active.password_hash = Set(new_password_hash.to_string());
        active.updated_at = Set(Utc::now());
        active.update(&self.db).await.map_err(db_err)?;

        Ok(())
    }

    async fn delete_user(&self, id: &str) -> DomainResult<()> {
        let result = user::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(db_err)?;

        if result.rows_affected == 0 {
            return Err(DomainError::NotFound {
                entity: "User",
                field: "id",
                value: id.to_string(),
            });
        }

        Ok(())
    }
}
