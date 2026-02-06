//! API Key generation and management

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// API Key prefix for identification
const API_KEY_PREFIX: &str = "txocpp_";

/// API Key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    /// The API key ID (stored in database)
    pub id: String,
    /// Name/description of the key
    pub name: String,
    /// Key prefix (for display, e.g., "txocpp_abc...")
    pub prefix: String,
    /// Hashed key (for verification)
    pub key_hash: String,
    /// Associated user ID (optional)
    pub user_id: Option<String>,
    /// Permissions/scopes
    pub scopes: Vec<String>,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Expiration time (optional)
    pub expires_at: Option<DateTime<Utc>>,
    /// Last used time
    pub last_used_at: Option<DateTime<Utc>>,
    /// Is the key active
    pub is_active: bool,
}

/// Result of API key generation
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedApiKey {
    /// The full API key (only shown once!)
    pub key: String,
    /// The key info (without the full key)
    pub info: ApiKeyInfo,
}

/// Generate a new API key
pub fn generate_api_key(name: &str, user_id: Option<&str>, scopes: Vec<String>) -> GeneratedApiKey {
    let mut rng = rand::thread_rng();
    
    // Generate random bytes for the key
    let random_bytes: [u8; 32] = rng.gen();
    let key_suffix = hex::encode(random_bytes);
    
    // Full key: prefix + random hex
    let full_key = format!("{}{}", API_KEY_PREFIX, key_suffix);
    
    // Hash the key for storage
    let key_hash = hash_api_key(&full_key);
    
    // Create key info
    let info = ApiKeyInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        prefix: format!("{}{}...", API_KEY_PREFIX, &key_suffix[..8]),
        key_hash,
        user_id: user_id.map(|s| s.to_string()),
        scopes,
        created_at: Utc::now(),
        expires_at: None,
        last_used_at: None,
        is_active: true,
    };
    
    GeneratedApiKey {
        key: full_key,
        info,
    }
}

/// Hash an API key for storage
pub fn hash_api_key(key: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    // For API keys, we use a simple hash since we need fast lookups
    // In production, consider using SHA-256 or similar
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Verify an API key against a stored hash
pub fn verify_api_key(key: &str, stored_hash: &str) -> bool {
    hash_api_key(key) == stored_hash
}

/// Check if a string looks like an API key
pub fn is_api_key_format(s: &str) -> bool {
    s.starts_with(API_KEY_PREFIX) && s.len() > API_KEY_PREFIX.len() + 32
}

/// Available API key scopes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiScope {
    /// Read charge point information
    ChargePointsRead,
    /// Manage charge points
    ChargePointsWrite,
    /// Read transactions
    TransactionsRead,
    /// Send commands to charge points
    CommandsExecute,
    /// Manage users (admin only)
    UsersManage,
    /// Full access
    Admin,
}

impl ApiScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChargePointsRead => "charge_points:read",
            Self::ChargePointsWrite => "charge_points:write",
            Self::TransactionsRead => "transactions:read",
            Self::CommandsExecute => "commands:execute",
            Self::UsersManage => "users:manage",
            Self::Admin => "admin",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "charge_points:read" => Some(Self::ChargePointsRead),
            "charge_points:write" => Some(Self::ChargePointsWrite),
            "transactions:read" => Some(Self::TransactionsRead),
            "commands:execute" => Some(Self::CommandsExecute),
            "users:manage" => Some(Self::UsersManage),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_api_key() {
        let result = generate_api_key("Test Key", Some("user-123"), vec!["admin".to_string()]);
        
        assert!(result.key.starts_with(API_KEY_PREFIX));
        assert!(is_api_key_format(&result.key));
        assert!(verify_api_key(&result.key, &result.info.key_hash));
        assert!(!verify_api_key("wrong-key", &result.info.key_hash));
    }
}
