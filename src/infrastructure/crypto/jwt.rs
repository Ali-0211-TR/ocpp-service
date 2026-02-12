//! JWT Token handling

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT configuration
#[derive(Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens
    pub secret: String,
    /// Token expiration time in hours
    pub expiration_hours: i64,
    /// Issuer claim
    pub issuer: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "super-secret-key-change-in-production".to_string()),
            expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(24),
            issuer: "texnouz-ocpp".to_string(),
        }
    }
}

impl JwtConfig {
    /// Create JwtConfig from environment variables
    pub fn from_env() -> Self {
        Self::default()
    }
}

/// JWT TokenClaims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// User role
    pub role: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Issuer
    pub iss: String,
}

impl TokenClaims {
    /// Create new Tokenclaims for a user
    pub fn new(user_id: &str, username: &str, role: &str, config: &JwtConfig) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(config.expiration_hours);

        Self {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            iss: config.issuer.clone(),
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// Check if the user has admin role
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

/// Create a JWT token for a user
pub fn create_token(
    user_id: &str,
    username: &str,
    role: &str,
    config: &JwtConfig,
) -> Result<String, jsonwebtoken::errors::Error> {
    let token_claims = TokenClaims::new(user_id, username, role, config);

    encode(
        &Header::default(),
        &token_claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
}

/// Verify and decode a JWT token
pub fn verify_token(
    token: &str,
    config: &JwtConfig,
) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.set_issuer(&[&config.issuer]);

    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}
