//! Reservation HTTP handlers

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::DateTime;

use crate::application::charging::commands::SharedCommandDispatcher;
use crate::application::charging::session::SharedSessionRegistry;
use crate::domain::reservation::{Reservation, ReservationStatus};
use crate::domain::RepositoryProvider;
use crate::interfaces::http::common::ApiResponse;

use super::dto::*;

/// Application state for reservation handlers.
#[derive(Clone)]
pub struct ReservationAppState {
    pub repos: Arc<dyn RepositoryProvider>,
    pub session_registry: SharedSessionRegistry,
    pub command_dispatcher: SharedCommandDispatcher,
}

#[utoipa::path(
    post,
    path = "/api/v1/reservations",
    tag = "Reservations",
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = CreateReservationRequest,
    responses(
        (status = 200, description = "Reservation result", body = ApiResponse<CreateReservationResponse>),
        (status = 404, description = "Charge point not connected"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_reservation(
    State(state): State<ReservationAppState>,
    Json(request): Json<CreateReservationRequest>,
) -> Result<
    Json<ApiResponse<CreateReservationResponse>>,
    (StatusCode, Json<ApiResponse<CreateReservationResponse>>),
> {
    // Validate charge point is connected
    if !state.session_registry.is_connected(&request.charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                request.charge_point_id
            ))),
        ));
    }

    // Parse expiry date
    let expiry_date = DateTime::parse_from_rfc3339(&request.expiry_date)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(format!("Invalid expiry_date: {}", e))),
            )
        })?;

    // Validate expiry is in the future
    if expiry_date <= chrono::Utc::now() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("expiry_date must be in the future")),
        ));
    }

    // Generate reservation ID
    let reservation_id = state.repos.reservations().next_id().await;

    // Send ReserveNow to the charge point
    let status = match state
        .command_dispatcher
        .reserve_now(
            &request.charge_point_id,
            reservation_id,
            request.connector_id,
            &request.id_tag,
            request.parent_id_tag.as_deref(),
            expiry_date,
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            ));
        }
    };

    // If accepted, persist the reservation
    let accepted = status.contains("Accepted");
    if accepted {
        let reservation = Reservation::new(
            reservation_id,
            &request.charge_point_id,
            request.connector_id,
            &request.id_tag,
            request.parent_id_tag.clone(),
            expiry_date,
        );
        if let Err(e) = state.repos.reservations().save(reservation).await {
            tracing::error!("Failed to save reservation: {}", e);
        }
    }

    Ok(Json(ApiResponse::success(CreateReservationResponse {
        reservation_id,
        status: status.clone(),
        message: if accepted {
            Some("Reservation created successfully".to_string())
        } else {
            Some(format!("Charge point responded: {}", status))
        },
    })))
}

#[utoipa::path(
    delete,
    path = "/api/v1/reservations/{reservation_id}",
    tag = "Reservations",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("reservation_id" = i32, Path, description = "Reservation ID")),
    responses(
        (status = 200, description = "Cancellation result", body = ApiResponse<CancelReservationResponse>),
        (status = 404, description = "Reservation not found")
    )
)]
pub async fn cancel_reservation(
    State(state): State<ReservationAppState>,
    Path(reservation_id): Path<i32>,
) -> Result<
    Json<ApiResponse<CancelReservationResponse>>,
    (StatusCode, Json<ApiResponse<CancelReservationResponse>>),
> {
    // Find the reservation
    let reservation = state
        .repos
        .reservations()
        .find_by_id(reservation_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let Some(reservation) = reservation else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Reservation {} not found",
                reservation_id
            ))),
        ));
    };

    // Can only cancel active reservations
    if reservation.status != ReservationStatus::Accepted {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Reservation {} is not active (status: {})",
                reservation_id, reservation.status
            ))),
        ));
    }

    // Send CancelReservation to charge point (if connected)
    let status = if state
        .session_registry
        .is_connected(&reservation.charge_point_id)
    {
        match state
            .command_dispatcher
            .cancel_reservation(&reservation.charge_point_id, reservation_id)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(e.to_string())),
                ));
            }
        }
    } else {
        // Charge point is offline — cancel locally only
        "Accepted".to_string()
    };

    // Update reservation status in DB
    let accepted = status.contains("Accepted");
    if accepted {
        if let Err(e) = state.repos.reservations().cancel(reservation_id).await {
            tracing::error!("Failed to cancel reservation in DB: {}", e);
        }
    }

    Ok(Json(ApiResponse::success(CancelReservationResponse {
        status: status.clone(),
        message: if accepted {
            Some("Reservation cancelled successfully".to_string())
        } else {
            Some(format!("Charge point responded: {}", status))
        },
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/reservations",
    tag = "Reservations",
    security(("bearer_auth" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "All reservations", body = ApiResponse<Vec<ReservationDto>>)
    )
)]
pub async fn list_reservations(
    State(state): State<ReservationAppState>,
) -> Result<
    Json<ApiResponse<Vec<ReservationDto>>>,
    (StatusCode, Json<ApiResponse<Vec<ReservationDto>>>),
> {
    let reservations = state.repos.reservations().find_all().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    let dtos: Vec<ReservationDto> = reservations
        .into_iter()
        .map(|r| ReservationDto {
            id: r.id,
            charge_point_id: r.charge_point_id,
            connector_id: r.connector_id,
            id_tag: r.id_tag,
            parent_id_tag: r.parent_id_tag,
            expiry_date: r.expiry_date.to_rfc3339(),
            status: r.status.as_str().to_string(),
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ApiResponse::success(dtos)))
}

#[utoipa::path(
    get,
    path = "/api/v1/reservations/{reservation_id}",
    tag = "Reservations",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("reservation_id" = i32, Path, description = "Reservation ID")),
    responses(
        (status = 200, description = "Reservation details", body = ApiResponse<ReservationDto>),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_reservation(
    State(state): State<ReservationAppState>,
    Path(reservation_id): Path<i32>,
) -> Result<
    Json<ApiResponse<ReservationDto>>,
    (StatusCode, Json<ApiResponse<ReservationDto>>),
> {
    let reservation = state
        .repos
        .reservations()
        .find_by_id(reservation_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let Some(r) = reservation else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Reservation {} not found",
                reservation_id
            ))),
        ));
    };

    Ok(Json(ApiResponse::success(ReservationDto {
        id: r.id,
        charge_point_id: r.charge_point_id,
        connector_id: r.connector_id,
        id_tag: r.id_tag,
        parent_id_tag: r.parent_id_tag,
        expiry_date: r.expiry_date.to_rfc3339(),
        status: r.status.as_str().to_string(),
        created_at: r.created_at.to_rfc3339(),
    })))
}
