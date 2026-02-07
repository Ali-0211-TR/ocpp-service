//! Authentication API handlers

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::ApiResponse;
use crate::auth::{create_token, hash_password, verify_password, JwtConfig};
use crate::infrastructure::database::entities::user;

/// Auth state for authentication handlers
#[derive(Clone)]
pub struct AuthHandlerState {
    pub db: sea_orm::DatabaseConnection,
    pub jwt_config: JwtConfig,
}

/// Запрос на авторизацию
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "username": "admin",
    "password": "secret123"
}))]
pub struct LoginRequest {
    /// Имя пользователя или email
    pub username: String,
    /// Пароль
    pub password: String,
}

/// Ответ на успешную авторизацию
///
/// Содержит JWT-токен для последующих запросов.
/// Токен передаётся в заголовке `Authorization: Bearer <token>`
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "token_type": "Bearer",
    "expires_in": 86400,
    "user": {
        "id": "user-123",
        "username": "admin",
        "email": "admin@example.com",
        "role": "admin"
    }
}))]
pub struct LoginResponse {
    /// JWT access-токен для авторизации. Передавайте в заголовке `Authorization: Bearer <token>`
    pub token: String,
    /// Тип токена (всегда `Bearer`)
    pub token_type: String,
    /// Время жизни токена в секундах (по умолчанию 86400 = 24 часа)
    pub expires_in: i64,
    /// Информация о пользователе
    pub user: UserInfo,
}

/// Информация о пользователе
#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    /// Уникальный идентификатор пользователя (UUID)
    pub id: String,
    /// Имя пользователя
    pub username: String,
    /// Email
    pub email: String,
    /// Роль: `admin`, `operator`, `viewer`
    pub role: String,
}

/// Запрос на регистрацию нового пользователя
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "username": "newuser",
    "email": "user@example.com",
    "password": "secure_password_123"
}))]
pub struct RegisterRequest {
    /// Имя пользователя (от 3 до 50 символов, уникальное)
    pub username: String,
    /// Email-адрес (уникальный)
    pub email: String,
    /// Пароль (минимум 8 символов)
    pub password: String,
}

/// Авторизация пользователя
///
/// Возвращает JWT-токен при успешной аутентификации.
/// Можно использовать как имя пользователя, так и email в поле `username`.
/// Если аккаунт деактивирован — вернёт 401.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Успешная авторизация, возвращает JWT-токен", body = ApiResponse<LoginResponse>),
        (status = 401, description = "Неверные учётные данные или аккаунт деактивирован")
    )
)]
pub async fn login(
    State(state): State<AuthHandlerState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ApiResponse<LoginResponse>>)> {
    // Find user by username or email
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

    // Check if user is active
    if !user.is_active {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Account is disabled")),
        ));
    }

    // Verify password
    let password_valid = verify_password(&request.password, &user.password_hash).unwrap_or(false);
    if !password_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Invalid credentials")),
        ));
    }

    // Update last login time
    let mut active_user: user::ActiveModel = user.clone().into();
    active_user.last_login_at = Set(Some(Utc::now()));
    active_user.update(&state.db).await.ok();

    // Create JWT token
    let role_str = match user.role {
        user::UserRole::Admin => "admin",
        user::UserRole::Operator => "operator",
        user::UserRole::Viewer => "viewer",
    };

    let token = create_token(&user.id, &user.username, role_str, &state.jwt_config).map_err(
        |e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        },
    )?;

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

/// Регистрация нового пользователя
///
/// Создаёт нового пользователя с ролью `viewer` (по умолчанию).
/// Логин и email должны быть уникальными.
/// Пароль: минимум 8 символов. Имя: 3–50 символов.
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Authentication",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Пользователь успешно создан", body = ApiResponse<UserInfo>),
        (status = 400, description = "Ошибка валидации (короткий пароль, невалидный email и т.д.)"),
        (status = 409, description = "Пользователь с таким логином или email уже существует")
    )
)]
pub async fn register(
    State(state): State<AuthHandlerState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<ApiResponse<UserInfo>>), (StatusCode, Json<ApiResponse<UserInfo>>)> {
    // Validate input
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

    // Check if username or email already exists
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

    // Hash password
    let password_hash = hash_password(&request.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    // Create user
    let now = Utc::now();
    let user_id = uuid::Uuid::new_v4().to_string();

    let new_user = user::ActiveModel {
        id: Set(user_id.clone()),
        username: Set(request.username.clone()),
        email: Set(request.email.clone()),
        password_hash: Set(password_hash),
        role: Set(user::UserRole::Viewer), // Default role
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

/// Получение информации о текущем пользователе
///
/// Возвращает данные пользователя, авторизованного по JWT-токену.
/// Используйте для проверки авторизации и получения роли.
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Authentication",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Информация о текущем пользователе", body = ApiResponse<UserInfo>),
        (status = 401, description = "Не авторизован (невалидный или отсутствующий токен)")
    )
)]
pub async fn get_current_user(
    State(state): State<AuthHandlerState>,
    user: Option<axum::Extension<crate::auth::middleware::AuthenticatedUser>>,
) -> Result<Json<ApiResponse<UserInfo>>, (StatusCode, Json<ApiResponse<UserInfo>>)> {
    let Some(user) = user else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Not authenticated")),
        ));
    };

    // Fetch full user info from database
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

/// Запрос на смену пароля
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    /// Текущий пароль для подтверждения
    pub current_password: String,
    /// Новый пароль (минимум 8 символов)
    pub new_password: String,
}

/// Смена пароля текущего пользователя
///
/// Для подтверждения операции требуется указать текущий пароль.
/// Новый пароль должен содержать минимум 8 символов.
#[utoipa::path(
    post,
    path = "/api/v1/auth/change-password",
    tag = "Authentication",
    security(
        ("bearer_auth" = [])
    ),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Пароль успешно изменён"),
        (status = 400, description = "Новый пароль слишком короткий (менее 8 символов)"),
        (status = 401, description = "Неверный текущий пароль или не авторизован")
    )
)]
pub async fn change_password(
    State(state): State<AuthHandlerState>,
    user: Option<axum::Extension<crate::auth::middleware::AuthenticatedUser>>,
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
            Json(ApiResponse::error("New password must be at least 8 characters")),
        ));
    }

    // Fetch user from database
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

    // Verify current password
    let password_valid =
        verify_password(&request.current_password, &db_user.password_hash).unwrap_or(false);
    if !password_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Invalid current password")),
        ));
    }

    // Hash new password
    let new_hash = hash_password(&request.new_password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    // Update password
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
