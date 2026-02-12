//! IdTag management handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

use super::dto::{CreateIdTagRequest, IdTagDto, ListIdTagsParams, UpdateIdTagRequest};
use crate::infrastructure::database::entities::id_tag::{self, IdTagStatus};
use crate::interfaces::http::common::{ApiResponse, PaginatedResponse};

/// IdTag handler state
#[derive(Clone)]
pub struct IdTagHandlerState {
    pub db: sea_orm::DatabaseConnection,
}

fn parse_status(s: &str) -> IdTagStatus {
    match s.to_lowercase().as_str() {
        "accepted" => IdTagStatus::Accepted,
        "blocked" => IdTagStatus::Blocked,
        "expired" => IdTagStatus::Expired,
        "invalid" => IdTagStatus::Invalid,
        "concurrenttx" => IdTagStatus::ConcurrentTx,
        _ => IdTagStatus::Accepted,
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/id-tags",
    tag = "IdTags",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(ListIdTagsParams),
    responses(
        (status = 200, description = "IdTag list", body = PaginatedResponse<IdTagDto>)
    )
)]
pub async fn list_id_tags(
    State(state): State<IdTagHandlerState>,
    Query(params): Query<ListIdTagsParams>,
) -> Result<Json<PaginatedResponse<IdTagDto>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut query = id_tag::Entity::find().order_by_desc(id_tag::Column::CreatedAt);

    if let Some(status) = &params.status {
        query = query.filter(id_tag::Column::Status.eq(parse_status(status)));
    }
    if let Some(is_active) = params.is_active {
        query = query.filter(id_tag::Column::IsActive.eq(is_active));
    }
    if let Some(user_id) = &params.user_id {
        query = query.filter(id_tag::Column::UserId.eq(user_id));
    }

    let total = query.clone().count(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    let page = params.page.max(1) as u32;
    let page_size = params.page_size.min(100).max(1) as u32;
    let offset = ((page - 1) * page_size) as u64;

    let tags = query
        .offset(offset)
        .limit(page_size as u64)
        .all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let items: Vec<IdTagDto> = tags.into_iter().map(IdTagDto::from).collect();
    Ok(Json(PaginatedResponse::new(items, total, page, page_size)))
}

#[utoipa::path(
    get,
    path = "/api/v1/id-tags/{id_tag}",
    tag = "IdTags",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("id_tag" = String, Path, description = "IdTag value")),
    responses(
        (status = 200, description = "IdTag details", body = ApiResponse<IdTagDto>),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_id_tag(
    State(state): State<IdTagHandlerState>,
    Path(id_tag_value): Path<String>,
) -> Result<Json<ApiResponse<IdTagDto>>, (StatusCode, Json<ApiResponse<IdTagDto>>)> {
    let tag = id_tag::Entity::find_by_id(&id_tag_value)
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    match tag {
        Some(t) => Ok(Json(ApiResponse::success(IdTagDto::from(t)))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("IdTag not found")),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/id-tags",
    tag = "IdTags",
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = CreateIdTagRequest,
    responses(
        (status = 201, description = "Created", body = ApiResponse<IdTagDto>),
        (status = 400, description = "Validation error"),
        (status = 409, description = "Already exists")
    )
)]
pub async fn create_id_tag(
    State(state): State<IdTagHandlerState>,
    Json(request): Json<CreateIdTagRequest>,
) -> Result<(StatusCode, Json<ApiResponse<IdTagDto>>), (StatusCode, Json<ApiResponse<IdTagDto>>)> {
    if request.id_tag.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("IdTag value cannot be empty")),
        ));
    }

    let existing = id_tag::Entity::find_by_id(&request.id_tag)
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
            Json(ApiResponse::error("IdTag already exists")),
        ));
    }

    let expiry_date: Option<DateTime<Utc>> = if let Some(ref expiry_str) = request.expiry_date {
        DateTime::parse_from_rfc3339(expiry_str)
            .map(|d| d.with_timezone(&Utc))
            .ok()
    } else {
        None
    };

    let now = Utc::now();
    let new_tag = id_tag::ActiveModel {
        id_tag: Set(request.id_tag),
        parent_id_tag: Set(request.parent_id_tag),
        status: Set(parse_status(&request.status)),
        user_id: Set(request.user_id),
        name: Set(request.name),
        expiry_date: Set(expiry_date),
        max_active_transactions: Set(request.max_active_transactions),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        last_used_at: Set(None),
    };

    let created = new_tag.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(IdTagDto::from(created))),
    ))
}

#[utoipa::path(
    put,
    path = "/api/v1/id-tags/{id_tag}",
    tag = "IdTags",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("id_tag" = String, Path, description = "IdTag value")),
    request_body = UpdateIdTagRequest,
    responses(
        (status = 200, description = "Updated", body = ApiResponse<IdTagDto>),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_id_tag(
    State(state): State<IdTagHandlerState>,
    Path(id_tag_value): Path<String>,
    Json(request): Json<UpdateIdTagRequest>,
) -> Result<Json<ApiResponse<IdTagDto>>, (StatusCode, Json<ApiResponse<IdTagDto>>)> {
    let tag = id_tag::Entity::find_by_id(&id_tag_value)
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let Some(tag) = tag else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("IdTag not found")),
        ));
    };

    let mut active: id_tag::ActiveModel = tag.into();
    active.updated_at = Set(Utc::now());

    if let Some(parent) = request.parent_id_tag {
        active.parent_id_tag = Set(Some(parent));
    }
    if let Some(status) = request.status {
        active.status = Set(parse_status(&status));
    }
    if let Some(user_id) = request.user_id {
        active.user_id = Set(Some(user_id));
    }
    if let Some(name) = request.name {
        active.name = Set(Some(name));
    }
    if let Some(expiry_str) = request.expiry_date {
        let expiry = DateTime::parse_from_rfc3339(&expiry_str)
            .map(|d| d.with_timezone(&Utc))
            .ok();
        active.expiry_date = Set(expiry);
    }
    if let Some(max_tx) = request.max_active_transactions {
        active.max_active_transactions = Set(Some(max_tx));
    }
    if let Some(is_active) = request.is_active {
        active.is_active = Set(is_active);
    }

    let updated = active.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    Ok(Json(ApiResponse::success(IdTagDto::from(updated))))
}

#[utoipa::path(
    delete,
    path = "/api/v1/id-tags/{id_tag}",
    tag = "IdTags",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("id_tag" = String, Path, description = "IdTag value")),
    responses(
        (status = 200, description = "Deleted"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_id_tag(
    State(state): State<IdTagHandlerState>,
    Path(id_tag_value): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let result = id_tag::Entity::delete_by_id(&id_tag_value)
        .exec(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    if result.rows_affected == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("IdTag not found")),
        ));
    }

    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    post,
    path = "/api/v1/id-tags/{id_tag}/block",
    tag = "IdTags",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("id_tag" = String, Path, description = "IdTag value")),
    responses(
        (status = 200, description = "Blocked", body = ApiResponse<IdTagDto>),
        (status = 404, description = "Not found")
    )
)]
pub async fn block_id_tag(
    State(state): State<IdTagHandlerState>,
    Path(id_tag_value): Path<String>,
) -> Result<Json<ApiResponse<IdTagDto>>, (StatusCode, Json<ApiResponse<IdTagDto>>)> {
    let tag = id_tag::Entity::find_by_id(&id_tag_value)
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let Some(tag) = tag else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("IdTag not found")),
        ));
    };

    let mut active: id_tag::ActiveModel = tag.into();
    active.status = Set(IdTagStatus::Blocked);
    active.updated_at = Set(Utc::now());

    let updated = active.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    Ok(Json(ApiResponse::success(IdTagDto::from(updated))))
}

#[utoipa::path(
    post,
    path = "/api/v1/id-tags/{id_tag}/unblock",
    tag = "IdTags",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("id_tag" = String, Path, description = "IdTag value")),
    responses(
        (status = 200, description = "Unblocked", body = ApiResponse<IdTagDto>),
        (status = 404, description = "Not found")
    )
)]
pub async fn unblock_id_tag(
    State(state): State<IdTagHandlerState>,
    Path(id_tag_value): Path<String>,
) -> Result<Json<ApiResponse<IdTagDto>>, (StatusCode, Json<ApiResponse<IdTagDto>>)> {
    let tag = id_tag::Entity::find_by_id(&id_tag_value)
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let Some(tag) = tag else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("IdTag not found")),
        ));
    };

    let mut active: id_tag::ActiveModel = tag.into();
    active.status = Set(IdTagStatus::Accepted);
    active.updated_at = Set(Utc::now());

    let updated = active.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    Ok(Json(ApiResponse::success(IdTagDto::from(updated))))
}
