#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub prefix: String,
    pub user_id: Option<String>,
    pub scopes: Vec<String>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}
