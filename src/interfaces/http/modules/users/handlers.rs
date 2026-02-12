//! User management API handlers
//!
//! Admin-only CRUD endpoints for managing users.
//! Delegates to `UserService` from the application/identity layer.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use super::dto::{CreateUserRequest, ListUsersParams, UpdateUserRequest, UserDto};
use crate::application::identity::{str_to_role, UserService};
use crate::domain::{GetUserDto, UpdateUserDto};
use crate::infrastructure::database::repositories::user_repository::UserRepository;
use crate::interfaces::http::common::{ApiResponse, PaginatedResponse};

/// User handler state — concrete over `UserRepository` for Axum compatibility.
#[derive(Clone)]
pub struct UserHandlerState {
    pub user_service: Arc<UserService<UserRepository>>,
}

#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "Users",
    security(("bearer_auth" = [])),
    params(ListUsersParams),
    responses(
        (status = 200, description = "User list", body = PaginatedResponse<UserDto>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_users(
    State(state): State<UserHandlerState>,
    Query(params): Query<ListUsersParams>,
) -> Result<Json<PaginatedResponse<UserDto>>, (StatusCode, Json<ApiResponse<()>>)> {
    let dto = GetUserDto {
        search: params.search,
        role: params.role.as_deref().map(str_to_role),
        page: Some(params.page),
        page_size: Some(params.page_size),
        sort_by: params.sort_by,
    };

    match state.user_service.list_users(dto).await {
        Ok(result) => {
            let items: Vec<UserDto> = result.items.into_iter().map(UserDto::from).collect();
            Ok(Json(PaginatedResponse::new(
                items,
                result.total,
                result.page,
                result.limit,
            )))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    tag = "Users",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "User ID")),
    responses(
        (status = 200, description = "User details", body = ApiResponse<UserDto>),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_user(
    State(state): State<UserHandlerState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<UserDto>>, (StatusCode, Json<ApiResponse<UserDto>>)> {
    match state.user_service.get_user_by_id(&id).await {
        Ok(Some(user)) => Ok(Json(ApiResponse::success(UserDto::from(user)))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("User '{}' not found", id))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "Users",
    security(("bearer_auth" = [])),
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = ApiResponse<UserDto>),
        (status = 400, description = "Validation error"),
        (status = 409, description = "Already exists")
    )
)]
pub async fn create_user(
    State(state): State<UserHandlerState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<ApiResponse<UserDto>>), (StatusCode, Json<ApiResponse<UserDto>>)> {
    match state
        .user_service
        .register(&request.username, &request.email, &request.password)
        .await
    {
        Ok(user) => Ok((
            StatusCode::CREATED,
            Json(ApiResponse::success(UserDto::from(user))),
        )),
        Err(e) => {
            let status = match &e {
                crate::domain::DomainError::Validation(_) => StatusCode::BAD_REQUEST,
                crate::domain::DomainError::Conflict(_) => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((status, Json(ApiResponse::error(e.to_string()))))
        }
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/users/{id}",
    tag = "Users",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "User ID")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = ApiResponse<UserDto>),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_user(
    State(state): State<UserHandlerState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<UserDto>>, (StatusCode, Json<ApiResponse<UserDto>>)> {
    let dto = UpdateUserDto {
        username: request.username,
        email: request.email,
    };

    match state.user_service.update_user(&id, dto).await {
        Ok(Some(user)) => Ok(Json(ApiResponse::success(UserDto::from(user)))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("User '{}' not found", id))),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    tag = "Users",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "User ID")),
    responses(
        (status = 200, description = "User deleted"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_user(
    State(state): State<UserHandlerState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.user_service.delete_user(&id).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => {
            let status = match &e {
                crate::domain::DomainError::NotFound { .. } => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((status, Json(ApiResponse::error(e.to_string()))))
        }
    }
}
