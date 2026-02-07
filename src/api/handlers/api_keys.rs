//! API Key management handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use sea_orm::prelude::Expr;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::ApiResponse;
use crate::auth::api_key::{generate_api_key, hash_api_key};
use crate::auth::middleware::AuthenticatedUser;
use crate::infrastructure::database::entities::api_key;

/// API key state
#[derive(Clone)]
pub struct ApiKeyHandlerState {
    pub db: sea_orm::DatabaseConnection,
}

/// Запрос на создание API-ключа
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "My Integration",
    "scopes": ["charge_points:read", "transactions:read"]
}))]
pub struct CreateApiKeyRequest {
    /// Название/описание ключа (для идентификации в списке)
    pub name: String,
    /// Права доступа (scopes). Примеры: `charge_points:read`, `charge_points:write`, `transactions:read`, `commands:execute`
    pub scopes: Vec<String>,
    /// Срок действия ключа в днях. null = бессрочно
    pub expires_in_days: Option<i64>,
}

/// API-ключ (ответ)
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyResponse {
    /// Уникальный ID ключа
    pub id: String,
    /// Название ключа
    pub name: String,
    /// Префикс ключа (первые 8 символов для идентификации)
    pub prefix: String,
    /// Права доступа
    pub scopes: Vec<String>,
    /// Активен ли ключ (false после отзыва)
    pub is_active: bool,
    /// Дата создания (ISO 8601)
    pub created_at: String,
    /// Дата истечения (ISO 8601). null = бессрочный
    pub expires_at: Option<String>,
    /// Последнее использование (ISO 8601)
    pub last_used_at: Option<String>,
}

/// Ответ при создании API-ключа
///
/// **ВАЖНО**: Полный ключ показывается ТОЛЬКО ОДИН РАЗ.
/// Сохраните его сразу, позже восстановить невозможно.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedApiKeyResponse {
    /// Полный API-ключ. **Сохраните сразу** — повторно получить нельзя!
    pub key: String,
    /// Метаданные ключа
    pub api_key: ApiKeyResponse,
}

/// Создание нового API-ключа
///
/// Генерирует новый API-ключ для программного доступа к системе.
/// Ключ передаётся в заголовке `X-API-Key`.
/// **Полный ключ показывается только один раз!**
#[utoipa::path(
    post,
    path = "/api/v1/api-keys",
    tag = "API Keys",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API-ключ создан. Сохраните полный ключ!", body = ApiResponse<CreatedApiKeyResponse>),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn create_api_key(
    State(state): State<ApiKeyHandlerState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<
    (StatusCode, Json<ApiResponse<CreatedApiKeyResponse>>),
    (StatusCode, Json<ApiResponse<CreatedApiKeyResponse>>),
> {
    // Generate new API key
    let generated = generate_api_key(&request.name, Some(&user.user_id), request.scopes.clone());

    // Calculate expiration
    let expires_at = request
        .expires_in_days
        .map(|days| Utc::now() + chrono::Duration::days(days));

    // Save to database
    let now = Utc::now();
    let new_key = api_key::ActiveModel {
        id: Set(generated.info.id.clone()),
        name: Set(request.name),
        key_hash: Set(generated.info.key_hash.clone()),
        prefix: Set(generated.info.prefix.clone()),
        user_id: Set(Some(user.user_id)),
        scopes: Set(serde_json::to_string(&request.scopes).unwrap_or_default()),
        is_active: Set(true),
        created_at: Set(now),
        expires_at: Set(expires_at),
        last_used_at: Set(None),
    };

    new_key.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    let response = CreatedApiKeyResponse {
        key: generated.key,
        api_key: ApiKeyResponse {
            id: generated.info.id,
            name: generated.info.name,
            prefix: generated.info.prefix,
            scopes: request.scopes,
            is_active: true,
            created_at: now.to_rfc3339(),
            expires_at: expires_at.map(|t| t.to_rfc3339()),
            last_used_at: None,
        },
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

/// Список API-ключей текущего пользователя
///
/// Возвращает все ключи пользователя (без самого секрета ключа).
/// Показывается префикс, скопы, статус и даты.
#[utoipa::path(
    get,
    path = "/api/v1/api-keys",
    tag = "API Keys",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Список API-ключей (без секретов)", body = ApiResponse<Vec<ApiKeyResponse>>),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn list_api_keys(
    State(state): State<ApiKeyHandlerState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<ApiResponse<Vec<ApiKeyResponse>>>, (StatusCode, Json<ApiResponse<Vec<ApiKeyResponse>>>)>
{
    let keys = api_key::Entity::find()
        .filter(api_key::Column::UserId.eq(&user.user_id))
        .all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let response: Vec<ApiKeyResponse> = keys
        .into_iter()
        .map(|k| {
            let scopes: Vec<String> = serde_json::from_str(&k.scopes).unwrap_or_default();
            ApiKeyResponse {
                id: k.id,
                name: k.name,
                prefix: k.prefix,
                scopes,
                is_active: k.is_active,
                created_at: k.created_at.to_rfc3339(),
                expires_at: k.expires_at.map(|t| t.to_rfc3339()),
                last_used_at: k.last_used_at.map(|t| t.to_rfc3339()),
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(response)))
}

/// Отзыв (деактивация) API-ключа
///
/// Отзывает ключ — он становится неактивным и не может быть использован для аутентификации.
/// Операция необратима.
#[utoipa::path(
    delete,
    path = "/api/v1/api-keys/{id}",
    tag = "API Keys",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = String, Path, description = "ID API-ключа для отзыва")
    ),
    responses(
        (status = 200, description = "API-ключ успешно отозван"),
        (status = 404, description = "API-ключ не найден")
    )
)]
pub async fn revoke_api_key(
    State(state): State<ApiKeyHandlerState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Find the key
    let key = api_key::Entity::find_by_id(&id)
        .filter(api_key::Column::UserId.eq(&user.user_id))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let Some(key) = key else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("API key not found")),
        ));
    };

    // Deactivate the key
    let mut active_key: api_key::ActiveModel = key.into();
    active_key.is_active = Set(false);
    active_key.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    Ok(Json(ApiResponse::success(())))
}

/// Verify and get API key info from hash (internal use)
pub async fn verify_api_key_from_db(
    db: &sea_orm::DatabaseConnection,
    api_key_str: &str,
) -> Option<(api_key::Model, Option<String>)> {
    let key_hash = hash_api_key(api_key_str);

    let key = api_key::Entity::find()
        .filter(api_key::Column::KeyHash.eq(&key_hash))
        .filter(api_key::Column::IsActive.eq(true))
        .one(db)
        .await
        .ok()??;

    // Check expiration
    if let Some(expires_at) = key.expires_at {
        if Utc::now() > expires_at {
            return None;
        }
    }

    // Update last used time (fire and forget)
    let key_id = key.id.clone();
    let user_id = key.user_id.clone();
    let db_clone = db.clone();
    tokio::spawn(async move {
        let _ = api_key::Entity::update_many()
            .filter(api_key::Column::Id.eq(&key_id))
            .col_expr(api_key::Column::LastUsedAt, Expr::value(Utc::now()))
            .exec(&db_clone)
            .await;
    });

    Some((key, user_id))
}
