//! User management service — application-layer orchestration
//!
//! All user-related business logic lives here.
//! HTTP handlers should be thin wrappers that delegate to this service.

use std::sync::Arc;

use tracing::info;

use crate::domain::{
    CreateUserDto, GetUserDto, UpdateUserDto, UserRepositoryInterface,
    DomainError, DomainResult, User, UserRole,
};
use crate::infrastructure::crypto::jwt::{create_token, JwtConfig};
use crate::infrastructure::crypto::password::{hash_password, verify_password};
use crate::shared::PaginatedResult;

/// Authentication result returned after a successful login
#[derive(Debug, Clone)]
pub struct AuthResult {
    pub token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: User,
}

/// User service — orchestrates all identity / user-management use-cases.
///
/// Generic over `R: UserRepositoryInterface` so it stays decoupled from
/// the concrete persistence layer.
pub struct UserService<R: UserRepositoryInterface> {
    repo: Arc<R>,
    jwt_config: JwtConfig,
}

impl<R: UserRepositoryInterface> UserService<R> {
    pub fn new(repo: Arc<R>, jwt_config: JwtConfig) -> Self {
        Self { repo, jwt_config }
    }

    // ── Authentication ──────────────────────────────────────────

    /// Authenticate user by username/email + password and return a JWT.
    pub async fn login(&self, username_or_email: &str, password: &str) -> DomainResult<AuthResult> {
        // Try username first, then email
        let user = self
            .repo
            .get_user_by_username(username_or_email)
            .await?
            .or(self.repo.get_user_by_email(username_or_email).await?);

        let Some(user) = user else {
            return Err(DomainError::Unauthorized("Invalid credentials".into()));
        };

        if !user.is_active {
            return Err(DomainError::Unauthorized("Account is disabled".into()));
        }

        let valid = verify_password(password, &user.password_hash).unwrap_or(false);
        if !valid {
            return Err(DomainError::Unauthorized("Invalid credentials".into()));
        }

        let role_str = role_to_str(&user.role);

        let token = create_token(&user.id, &user.username, role_str, &self.jwt_config)
            .map_err(|e| DomainError::Validation(format!("Failed to create token: {}", e)))?;

        Ok(AuthResult {
            token,
            token_type: "Bearer".into(),
            expires_in: self.jwt_config.expiration_hours * 3600,
            user,
        })
    }

    // ── Registration ────────────────────────────────────────────

    /// Register a new user (default role: Viewer).
    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> DomainResult<User> {
        // Validation
        if username.len() < 3 || username.len() > 50 {
            return Err(DomainError::Validation(
                "Username must be 3-50 characters".into(),
            ));
        }
        if password.len() < 8 {
            return Err(DomainError::Validation(
                "Password must be at least 8 characters".into(),
            ));
        }
        if !email.contains('@') {
            return Err(DomainError::Validation("Invalid email address".into()));
        }

        // Check uniqueness
        if self.repo.get_user_by_username(username).await?.is_some() {
            return Err(DomainError::Conflict("Username already exists".into()));
        }
        if self.repo.get_user_by_email(email).await?.is_some() {
            return Err(DomainError::Conflict("Email already exists".into()));
        }

        let dto = CreateUserDto {
            username: username.to_string(),
            email: email.to_string(),
            role: None, // default Viewer
            password: password.to_string(),
        };

        self.repo.create_user(dto).await?;

        // Fetch the newly created user
        let user = self
            .repo
            .get_user_by_username(username)
            .await?
            .ok_or_else(|| {
                DomainError::Validation("User created but could not be retrieved".into())
            })?;

        info!(user_id = %user.id, username = %user.username, "New user registered");
        Ok(user)
    }

    // ── Queries ─────────────────────────────────────────────────

    /// List users with search, filtering, sorting and pagination.
    pub async fn list_users(&self, dto: GetUserDto) -> DomainResult<PaginatedResult<User>> {
        self.repo.list_users(dto).await
    }

    /// Get a single user by ID.
    pub async fn get_user_by_id(&self, id: &str) -> DomainResult<Option<User>> {
        self.repo.get_user_by_id(id).await
    }

    /// Get user by username.
    pub async fn get_user_by_username(&self, username: &str) -> DomainResult<Option<User>> {
        self.repo.get_user_by_username(username).await
    }

    /// Get user by email.
    pub async fn get_user_by_email(&self, email: &str) -> DomainResult<Option<User>> {
        self.repo.get_user_by_email(email).await
    }

    // ── Commands (mutations) ────────────────────────────────────

    /// Update user profile fields (username, email).
    pub async fn update_user(&self, id: &str, dto: UpdateUserDto) -> DomainResult<Option<User>> {
        self.repo.update_user(id, dto).await
    }

    /// Change a user's password. Verifies the current password first.
    pub async fn change_password(
        &self,
        user_id: &str,
        current_password: &str,
        new_password: &str,
    ) -> DomainResult<()> {
        if new_password.len() < 8 {
            return Err(DomainError::Validation(
                "New password must be at least 8 characters".into(),
            ));
        }

        let user = self
            .repo
            .get_user_by_id(user_id)
            .await?
            .ok_or(DomainError::NotFound {
                entity: "User",
                field: "id",
                value: user_id.to_string(),
            })?;

        let valid = verify_password(current_password, &user.password_hash).unwrap_or(false);
        if !valid {
            return Err(DomainError::Unauthorized("Invalid current password".into()));
        }

        let new_hash = hash_password(new_password)
            .map_err(|e| DomainError::Validation(format!("Failed to hash password: {}", e)))?;

        self.repo.update_user_password(user_id, &new_hash).await?;

        info!(user_id, "Password changed");
        Ok(())
    }

    /// Delete a user by ID.
    pub async fn delete_user(&self, id: &str) -> DomainResult<()> {
        self.repo.delete_user(id).await
    }
}

// ── Helpers ─────────────────────────────────────────────────────

pub fn role_to_str(role: &UserRole) -> &'static str {
    match role {
        UserRole::Admin => "admin",
        UserRole::Operator => "operator",
        UserRole::Viewer => "viewer",
    }
}

pub fn str_to_role(s: &str) -> UserRole {
    match s.to_lowercase().as_str() {
        "admin" => UserRole::Admin,
        "operator" => UserRole::Operator,
        _ => UserRole::Viewer,
    }
}
