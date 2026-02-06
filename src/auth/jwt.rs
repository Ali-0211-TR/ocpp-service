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

/// JWT Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
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

impl Claims {
    /// Create new claims for a user
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
    let claims = Claims::new(user_id, username, role, config);

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
}

/// Verify and decode a JWT token
pub fn verify_token(token: &str, config: &JwtConfig) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.set_issuer(&[&config.issuer]);

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

/// Errors that can occur during authentication
#[derive(Debug, Clone)]
pub enum AuthError {
    /// Token is missing
    MissingToken,
    /// Token is invalid
    InvalidToken,
    /// Token has expired
    ExpiredToken,
    /// Insufficient permissions
    InsufficientPermissions,
    /// Invalid credentials
    InvalidCredentials,
    /// User not found
    UserNotFound,
    /// API key is invalid
    InvalidApiKey,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingToken => write!(f, "Missing authentication token"),
            Self::InvalidToken => write!(f, "Invalid authentication token"),
            Self::ExpiredToken => write!(f, "Token has expired"),
            Self::InsufficientPermissions => write!(f, "Insufficient permissions"),
            Self::InvalidCredentials => write!(f, "Invalid credentials"),
            Self::UserNotFound => write!(f, "User not found"),
            Self::InvalidApiKey => write!(f, "Invalid API key"),
        }
    }
}

impl std::error::Error for AuthError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_token() {
        let config = JwtConfig::default();
        let token = create_token("user-123", "testuser", "admin", &config).unwrap();

        let claims = verify_token(&token, &config).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.role, "admin");
        assert!(!claims.is_expired());
        assert!(claims.is_admin());
    }

    #[test]
    fn test_invalid_token() {
        let config = JwtConfig::default();
        let result = verify_token("invalid-token", &config);
        assert!(result.is_err());
    }
}
