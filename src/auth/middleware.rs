//! Authentication middleware for Axum

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use sea_orm::prelude::Expr;
use serde_json::json;

use super::api_key::{hash_api_key, is_api_key_format};
use super::jwt::{verify_token, AuthError, Claims, JwtConfig};
use crate::infrastructure::database::entities::api_key;
use crate::infrastructure::Storage;

/// Authentication state containing JWT config and storage
#[derive(Clone)]
pub struct AuthState {
    pub jwt_config: JwtConfig,
    pub storage: Arc<dyn Storage>,
    pub db: DatabaseConnection,
}

/// Authenticated user information (either from JWT or API key)
#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub auth_method: AuthMethod,
}

/// How the user was authenticated
#[derive(Clone, Debug)]
pub enum AuthMethod {
    /// JWT token
    Jwt,
    /// API key
    ApiKey { key_id: String },
}

impl AuthenticatedUser {
    pub fn from_claims(claims: Claims) -> Self {
        Self {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
            auth_method: AuthMethod::Jwt,
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

/// Extract token from Authorization header
fn extract_token(auth_header: &str) -> Option<&str> {
    if auth_header.starts_with("Bearer ") {
        Some(&auth_header[7..])
    } else {
        None
    }
}

/// JWT authentication middleware - requires valid token
pub async fn auth_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Get Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(String::from);
    let Some(auth_header) = auth_header else {
        return auth_error_response(AuthError::MissingToken);
    };

    // Try API key first
    if is_api_key_format(&auth_header) {
        return handle_api_key_auth(&auth_header, &auth_state, request, next).await;
    }

    // Try Bearer token
    let Some(token) = extract_token(&auth_header) else {
        return auth_error_response(AuthError::InvalidToken);
    };

    // Verify JWT token
    match verify_token(token, &auth_state.jwt_config) {
        Ok(claims) => {
            if claims.is_expired() {
                return auth_error_response(AuthError::ExpiredToken);
            }

            // Add authenticated user to request extensions
            let user = AuthenticatedUser::from_claims(claims);
            request.extensions_mut().insert(user);

            next.run(request).await
        }
        Err(_) => auth_error_response(AuthError::InvalidToken),
    }
}

/// Optional authentication middleware - allows unauthenticated requests
pub async fn optional_auth_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Get Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        // Try API key
        if is_api_key_format(auth_header) {
            // For optional auth, we don't fail on invalid API key
            if let Some(user) = try_api_key_auth(auth_header, &auth_state).await {
                request.extensions_mut().insert(user);
            }
        } else if let Some(token) = extract_token(auth_header) {
            // Try JWT
            if let Ok(claims) = verify_token(token, &auth_state.jwt_config) {
                if !claims.is_expired() {
                    let user = AuthenticatedUser::from_claims(claims);
                    request.extensions_mut().insert(user);
                }
            }
        }
    }

    next.run(request).await
}

/// API key only authentication middleware
pub async fn api_key_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Check X-API-Key header first
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            // Fallback to Authorization header
            request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
        });

    let Some(api_key) = api_key else {
        return auth_error_response(AuthError::MissingToken);
    };

    if !is_api_key_format(api_key) {
        return auth_error_response(AuthError::InvalidApiKey);
    }

    match try_api_key_auth(api_key, &auth_state).await {
        Some(user) => {
            request.extensions_mut().insert(user);
            next.run(request).await
        }
        None => auth_error_response(AuthError::InvalidApiKey),
    }
}

/// Handle API key authentication
async fn handle_api_key_auth(
    api_key: &str,
    auth_state: &AuthState,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    match try_api_key_auth(api_key, auth_state).await {
        Some(user) => {
            request.extensions_mut().insert(user);
            next.run(request).await
        }
        None => auth_error_response(AuthError::InvalidApiKey),
    }
}

/// Try to authenticate with API key
async fn try_api_key_auth(api_key_str: &str, auth_state: &AuthState) -> Option<AuthenticatedUser> {
    // Hash the API key for lookup
    let key_hash = hash_api_key(api_key_str);

    // Look up in database
    let key = api_key::Entity::find()
        .filter(api_key::Column::KeyHash.eq(&key_hash))
        .filter(api_key::Column::IsActive.eq(true))
        .one(&auth_state.db)
        .await
        .ok()??;

    // Check expiration
    if let Some(expires_at) = key.expires_at {
        if chrono::Utc::now() > expires_at {
            return None;
        }
    }

    // Update last used timestamp (fire and forget)
    let key_id = key.id.clone();
    let db = auth_state.db.clone();
    tokio::spawn(async move {
        let _ = api_key::Entity::update_many()
            .filter(api_key::Column::Id.eq(&key_id))
            .col_expr(
                api_key::Column::LastUsedAt,
                Expr::value(chrono::Utc::now()),
            )
            .exec(&db)
            .await;
    });

    // Get scopes
    let scopes: Vec<String> = serde_json::from_str(&key.scopes).unwrap_or_default();
    
    // Determine role from scopes (simplified - in production you'd check actual scopes)
    let role = if scopes.iter().any(|s| s.contains("admin")) {
        "admin"
    } else {
        "operator"
    };

    Some(AuthenticatedUser {
        user_id: key.user_id.unwrap_or_else(|| "api-key-user".to_string()),
        username: key.name,
        role: role.to_string(),
        auth_method: AuthMethod::ApiKey { key_id: key.id },
    })
}

/// Create an authentication error response
fn auth_error_response(error: AuthError) -> Response {
    let (status, message) = match error {
        AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
        AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid authentication token"),
        AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Token has expired"),
        AuthError::InsufficientPermissions => (StatusCode::FORBIDDEN, "Insufficient permissions"),
        AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
        AuthError::UserNotFound => (StatusCode::NOT_FOUND, "User not found"),
        AuthError::InvalidApiKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
    };

    let body = Json(json!({
        "success": false,
        "error": message
    }));

    (status, body).into_response()
}

/// Admin-only middleware - must be used after auth_middleware
pub async fn admin_middleware(request: Request<Body>, next: Next) -> Response {
    let user = request.extensions().get::<AuthenticatedUser>();

    match user {
        Some(user) if user.is_admin() => next.run(request).await,
        Some(_) => auth_error_response(AuthError::InsufficientPermissions),
        None => auth_error_response(AuthError::MissingToken),
    }
}
