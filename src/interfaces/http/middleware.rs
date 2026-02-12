//! Authentication middleware for Axum

use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::prelude::Expr;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::json;

use crate::infrastructure::crypto::api_key::hash_api_key;
use crate::infrastructure::crypto::jwt::{verify_token, JwtConfig, TokenClaims};
use crate::infrastructure::database::entities::api_key;

/// API key prefix
const API_KEY_PREFIX: &str = "txocpp_";

/// Check if a string looks like an API key
fn is_api_key_format(s: &str) -> bool {
    s.starts_with(API_KEY_PREFIX)
}

/// Authentication error types
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    ExpiredToken,
    InsufficientPermissions,
    InvalidCredentials,
    UserNotFound,
    InvalidApiKey,
}

/// Authentication state containing JWT config and storage
#[derive(Clone)]
pub struct AuthState {
    pub jwt_config: JwtConfig,
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
    Jwt,
    ApiKey { key_id: String },
}

impl AuthenticatedUser {
    pub fn from_claims(claims: TokenClaims) -> Self {
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

fn extract_token(auth_header: &str) -> Option<&str> {
    if auth_header.starts_with("Bearer ") {
        Some(&auth_header[7..])
    } else {
        None
    }
}

/// JWT / API-key authentication middleware
pub async fn auth_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
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

    match verify_token(token, &auth_state.jwt_config) {
        Ok(claims) => {
            if claims.is_expired() {
                return auth_error_response(AuthError::ExpiredToken);
            }
            let user = AuthenticatedUser::from_claims(claims);
            request.extensions_mut().insert(user);
            next.run(request).await
        }
        Err(_) => auth_error_response(AuthError::InvalidToken),
    }
}

/// Optional authentication middleware
#[allow(dead_code)]
pub async fn optional_auth_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        if is_api_key_format(auth_header) {
            if let Some(user) = try_api_key_auth(auth_header, &auth_state).await {
                request.extensions_mut().insert(user);
            }
        } else if let Some(token) = extract_token(auth_header) {
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

async fn try_api_key_auth(api_key_str: &str, auth_state: &AuthState) -> Option<AuthenticatedUser> {
    let key_hash = hash_api_key(api_key_str);

    let key = api_key::Entity::find()
        .filter(api_key::Column::KeyHash.eq(&key_hash))
        .filter(api_key::Column::IsActive.eq(true))
        .one(&auth_state.db)
        .await
        .ok()??;

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
            .col_expr(api_key::Column::LastUsedAt, Expr::value(chrono::Utc::now()))
            .exec(&db)
            .await;
    });

    let scopes: Vec<String> = serde_json::from_str(&key.scopes).unwrap_or_default();
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
