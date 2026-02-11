use chrono::{DateTime, Utc};

/// User role
#[derive(Debug, Clone)]
pub enum UserRole {
    Admin,
    Operator,
    Viewer,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::Viewer
    }
}

/// User model
#[derive(Clone, Debug)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}
