//! Authentication API handlers

use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use super::dto::{ChangePasswordRequest, LoginRequest, LoginResponse, RegisterRequest, UserInfo};
use crate::infrastructure::crypto::jwt::{create_token, JwtConfig};
use crate::infrastructure::crypto::password::{hash_password, verify_password};
use crate::infrastructure::database::entities::user;
use crate::interfaces::http::common::ApiResponse;
use crate::interfaces::http::middleware::AuthenticatedUser;

/// Auth state
#[derive(Clone)]
pub struct AuthHandlerState {
    pub db: sea_orm::DatabaseConnection,
    pub jwt_config: JwtConfig,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Successful login", body = ApiResponse<LoginResponse>),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn login(
    State(state): State<AuthHandlerState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ApiResponse<LoginResponse>>)> {
    let user = user::Entity::find()
        .filter(
            user::Column::Username
                .eq(&request.username)
                .or(user::Column::Email.eq(&request.username)),
        )
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let Some(user) = user else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Invalid credentials")),
        ));
    };

    if !user.is_active {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Account is disabled")),
        ));
    }

    let password_valid = verify_password(&request.password, &user.password_hash).unwrap_or(false);
    if !password_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Invalid credentials")),
        ));
    }

    let mut active_user: user::ActiveModel = user.clone().into();
    active_user.last_login_at = Set(Some(Utc::now()));
    active_user.update(&state.db).await.ok();

    let role_str = match user.role {
        user::UserRole::Admin => "admin",
        user::UserRole::Operator => "operator",
        user::UserRole::Viewer => "viewer",
    };

    let token =
        create_token(&user.id, &user.username, role_str, &state.jwt_config).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let response = LoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: state.jwt_config.expiration_hours * 3600,
        user: UserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
            role: role_str.to_string(),
        },
    };

    Ok(Json(ApiResponse::success(response)))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Authentication",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User created", body = ApiResponse<UserInfo>),
        (status = 400, description = "Validation error"),
        (status = 409, description = "User already exists")
    )
)]
pub async fn register(
    State(state): State<AuthHandlerState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<ApiResponse<UserInfo>>), (StatusCode, Json<ApiResponse<UserInfo>>)> {
    if request.username.len() < 3 || request.username.len() > 50 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Username must be 3-50 characters")),
        ));
    }

    if request.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Password must be at least 8 characters")),
        ));
    }

    if !request.email.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Invalid email address")),
        ));
    }

    let existing = user::Entity::find()
        .filter(
            user::Column::Username
                .eq(&request.username)
                .or(user::Column::Email.eq(&request.email)),
        )
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::error("Username or email already exists")),
        ));
    }

    let password_hash = hash_password(&request.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    let now = Utc::now();
    let user_id = uuid::Uuid::new_v4().to_string();

    let new_user = user::ActiveModel {
        id: Set(user_id.clone()),
        username: Set(request.username.clone()),
        email: Set(request.email.clone()),
        password_hash: Set(password_hash),
        role: Set(user::UserRole::Viewer),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(None),
    };

    new_user.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    let response = UserInfo {
        id: user_id,
        username: request.username,
        email: request.email,
        role: "viewer".to_string(),
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user info", body = ApiResponse<UserInfo>),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn get_current_user(
    State(state): State<AuthHandlerState>,
    user: Option<axum::Extension<AuthenticatedUser>>,
) -> Result<Json<ApiResponse<UserInfo>>, (StatusCode, Json<ApiResponse<UserInfo>>)> {
    let Some(user) = user else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Not authenticated")),
        ));
    };

    let db_user = user::Entity::find_by_id(&user.user_id)
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let Some(db_user) = db_user else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("User not found")),
        ));
    };

    let response = UserInfo {
        id: db_user.id,
        username: db_user.username,
        email: db_user.email,
        role: user.role.clone(),
    };

    Ok(Json(ApiResponse::success(response)))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/change-password",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed"),
        (status = 400, description = "New password too short"),
        (status = 401, description = "Invalid current password")
    )
)]
pub async fn change_password(
    State(state): State<AuthHandlerState>,
    user: Option<axum::Extension<AuthenticatedUser>>,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let Some(user) = user else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Not authenticated")),
        ));
    };

    if request.new_password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "New password must be at least 8 characters",
            )),
        ));
    }

    let db_user = user::Entity::find_by_id(&user.user_id)
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let Some(db_user) = db_user else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("User not found")),
        ));
    };

    let password_valid =
        verify_password(&request.current_password, &db_user.password_hash).unwrap_or(false);
    if !password_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Invalid current password")),
        ));
    }

    let new_hash = hash_password(&request.new_password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    let mut active_user: user::ActiveModel = db_user.into();
    active_user.password_hash = Set(new_hash);
    active_user.updated_at = Set(Utc::now());
    active_user.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    Ok(Json(ApiResponse::success(())))
}
