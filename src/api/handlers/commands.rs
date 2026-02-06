//! Remote command API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use crate::api::dto::{
    ApiResponse, ChangeAvailabilityRequest, CommandResponse, RemoteStartRequest,
    RemoteStopRequest, ResetRequest, TriggerMessageRequest, UnlockConnectorRequest,
};
use crate::application::{
    change_availability, get_configuration, remote_start_transaction, remote_stop_transaction,
    reset, trigger_message, unlock_connector, Availability, CommandSender, ResetKind, TriggerType,
};
use crate::session::SharedSessionManager;

/// Extended application state with command sender
#[derive(Clone)]
pub struct CommandAppState {
    pub session_manager: SharedSessionManager,
    pub command_sender: Arc<CommandSender>,
}

/// Remote start transaction
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/remote-start",
    tag = "Commands",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    request_body = RemoteStartRequest,
    responses(
        (status = 200, description = "Command result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Charge point offline or not found")
    )
)]
pub async fn remote_start(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<RemoteStartRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    // Check if charge point is online
    if !state.session_manager.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    // Send remote start command
    match remote_start_transaction(
        &state.command_sender,
        &charge_point_id,
        &request.id_tag,
        request.connector_id,
    )
    .await
    {
        Ok(status) => {
            let status_str = format!("{:?}", status);
            let accepted = status_str.contains("Accepted");
            Ok(Json(ApiResponse::success(CommandResponse {
                status: status_str,
                message: if accepted {
                    Some("Remote start accepted".to_string())
                } else {
                    Some("Remote start rejected by charge point".to_string())
                },
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Remote stop transaction
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/remote-stop",
    tag = "Commands",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    request_body = RemoteStopRequest,
    responses(
        (status = 200, description = "Command result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Charge point offline or not found")
    )
)]
pub async fn remote_stop(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<RemoteStopRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_manager.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    match remote_stop_transaction(&state.command_sender, &charge_point_id, request.transaction_id)
        .await
    {
        Ok(status) => {
            let status_str = format!("{:?}", status);
            let accepted = status_str.contains("Accepted");
            Ok(Json(ApiResponse::success(CommandResponse {
                status: status_str,
                message: if accepted {
                    Some("Remote stop accepted".to_string())
                } else {
                    Some("Remote stop rejected by charge point".to_string())
                },
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Reset charge point
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/reset",
    tag = "Commands",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    request_body = ResetRequest,
    responses(
        (status = 200, description = "Command result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Charge point offline or not found")
    )
)]
pub async fn reset_charge_point(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<ResetRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_manager.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    let reset_type = match request.reset_type.to_lowercase().as_str() {
        "hard" => ResetKind::Hard,
        _ => ResetKind::Soft,
    };

    match reset(&state.command_sender, &charge_point_id, reset_type).await {
        Ok(status) => {
            let status_str = format!("{:?}", status);
            let accepted = status_str.contains("Accepted");
            Ok(Json(ApiResponse::success(CommandResponse {
                status: status_str,
                message: if accepted {
                    Some("Reset accepted".to_string())
                } else {
                    Some("Reset rejected by charge point".to_string())
                },
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Unlock connector
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/unlock-connector",
    tag = "Commands",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    request_body = UnlockConnectorRequest,
    responses(
        (status = 200, description = "Command result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Charge point offline or not found")
    )
)]
pub async fn unlock(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<UnlockConnectorRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_manager.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    match unlock_connector(&state.command_sender, &charge_point_id, request.connector_id).await {
        Ok(status) => {
            let status_str = format!("{:?}", status);
            let unlocked = status_str.contains("Unlocked");
            Ok(Json(ApiResponse::success(CommandResponse {
                status: status_str,
                message: if unlocked {
                    Some("Connector unlocked".to_string())
                } else {
                    Some("Unlock failed".to_string())
                },
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Change connector availability
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/change-availability",
    tag = "Commands",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    request_body = ChangeAvailabilityRequest,
    responses(
        (status = 200, description = "Command result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Charge point offline or not found")
    )
)]
pub async fn change_avail(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<ChangeAvailabilityRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_manager.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    let availability = match request.availability_type.to_lowercase().as_str() {
        "inoperative" => Availability::Inoperative,
        _ => Availability::Operative,
    };

    match change_availability(
        &state.command_sender,
        &charge_point_id,
        request.connector_id,
        availability,
    )
    .await
    {
        Ok(status) => {
            let status_str = format!("{:?}", status);
            let accepted = status_str.contains("Accepted");
            let scheduled = status_str.contains("Scheduled");
            Ok(Json(ApiResponse::success(CommandResponse {
                status: status_str,
                message: if accepted {
                    Some("Availability changed".to_string())
                } else if scheduled {
                    Some("Availability change scheduled".to_string())
                } else {
                    Some("Availability change rejected".to_string())
                },
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Trigger message from charge point
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/trigger-message",
    tag = "Commands",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID")
    ),
    request_body = TriggerMessageRequest,
    responses(
        (status = 200, description = "Command result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Charge point offline or not found")
    )
)]
pub async fn trigger_msg(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<TriggerMessageRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_manager.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    let message_type = match request.message.to_lowercase().as_str() {
        "bootnotification" => TriggerType::BootNotification,
        "diagnosticsstatusnotification" => TriggerType::DiagnosticsStatusNotification,
        "firmwarestatusnotification" => TriggerType::FirmwareStatusNotification,
        "heartbeat" => TriggerType::Heartbeat,
        "metervalues" => TriggerType::MeterValues,
        "statusnotification" => TriggerType::StatusNotification,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(format!(
                    "Unknown message trigger: {}",
                    request.message
                ))),
            ));
        }
    };

    match trigger_message(
        &state.command_sender,
        &charge_point_id,
        message_type,
        request.connector_id,
    )
    .await
    {
        Ok(status) => {
            let status_str = format!("{:?}", status);
            let accepted = status_str.contains("Accepted");
            let not_implemented = status_str.contains("NotImplemented");
            Ok(Json(ApiResponse::success(CommandResponse {
                status: status_str,
                message: if accepted {
                    Some("Trigger message accepted".to_string())
                } else if not_implemented {
                    Some("Message trigger not implemented by charge point".to_string())
                } else {
                    Some("Trigger message rejected".to_string())
                },
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Get configuration from charge point
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/configuration",
    tag = "Commands",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID"),
        ("keys" = Option<String>, Query, description = "Comma-separated list of configuration keys")
    ),
    responses(
        (status = 200, description = "Configuration values", body = ApiResponse<ConfigurationResponse>),
        (status = 404, description = "Charge point offline or not found")
    )
)]
pub async fn get_config(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Query(params): Query<ConfigurationParams>,
) -> Result<
    Json<ApiResponse<ConfigurationResponse>>,
    (StatusCode, Json<ApiResponse<ConfigurationResponse>>),
> {
    if !state.session_manager.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    let keys: Option<Vec<String>> = params
        .keys
        .map(|k| k.split(',').map(|s| s.trim().to_string()).collect());

    match get_configuration(&state.command_sender, &charge_point_id, keys).await {
        Ok(result) => {
            let config_values: Vec<ConfigValue> = result
                .configuration_key
                .into_iter()
                .map(|k| ConfigValue {
                    key: k.key,
                    value: k.value,
                    readonly: k.readonly,
                })
                .collect();

            Ok(Json(ApiResponse::success(ConfigurationResponse {
                configuration: config_values,
                unknown_keys: result.unknown_key,
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Query parameters for configuration
#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct ConfigurationParams {
    /// Comma-separated list of configuration keys
    pub keys: Option<String>,
}

/// Configuration value
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ConfigValue {
    pub key: String,
    pub value: Option<String>,
    pub readonly: bool,
}

/// Configuration response
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ConfigurationResponse {
    pub configuration: Vec<ConfigValue>,
    pub unknown_keys: Vec<String>,
}
