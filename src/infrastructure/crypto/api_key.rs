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

/// Slugify a name for embedding in the API key.
/// Converts "My Integration" → "my-integration", max 24 chars.
fn slugify_name(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    // Collapse multiple dashes
    let mut prev_dash = false;
    let collapsed: String = slug
        .chars()
        .filter(|&c| {
            if c == '-' {
                if prev_dash {
                    return false;
                }
                prev_dash = true;
            } else {
                prev_dash = false;
            }
            true
        })
        .collect();
    // Trim dashes from edges, limit to 24 chars
    let trimmed = collapsed.trim_matches('-');
    if trimmed.len() > 24 {
        trimmed[..24].trim_end_matches('-').to_string()
    } else {
        trimmed.to_string()
    }
}

/// Generate a new API key with semantic format:
/// `txocpp_<name-slug>_<random-hex>`
///
/// Example: `txocpp_texnouz-gsms_a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4`
pub fn generate_api_key(name: &str, user_id: Option<&str>, scopes: Vec<String>) -> GeneratedApiKey {
    let mut rng = rand::thread_rng();

    // Generate random bytes for the key (16 bytes = 32 hex chars)
    let random_bytes: [u8; 16] = rng.gen();
    let random_hex = hex::encode(random_bytes);

    // Build semantic key: txocpp_<slug>_<random>
    let name_slug = slugify_name(name);
    let full_key = if name_slug.is_empty() {
        format!("{}{}", API_KEY_PREFIX, random_hex)
    } else {
        format!("{}{}_{}", API_KEY_PREFIX, name_slug, random_hex)
    };

    // Hash the key for storage
    let key_hash = hash_api_key(&full_key);

    // Create prefix for display (first 12 chars of random part)
    let display_prefix = if name_slug.is_empty() {
        format!("{}{}...", API_KEY_PREFIX, &random_hex[..8])
    } else {
        format!("{}{}_{}...", API_KEY_PREFIX, name_slug, &random_hex[..8])
    };

    // Create key info
    let info = ApiKeyInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        prefix: display_prefix,
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

/// Hash an API key for storage using SHA-256
pub fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Verify an API key against a stored hash
pub fn verify_api_key(key: &str, stored_hash: &str) -> bool {
    hash_api_key(key) == stored_hash
}
