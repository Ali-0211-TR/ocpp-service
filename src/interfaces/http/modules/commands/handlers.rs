//! Remote command API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use tracing::{error, info, warn};

use super::dto::{
    ChangeAvailabilityRequest, ChangeConfigurationRequest, ClearChargingProfileRequest,
    CommandResponse, DataTransferRequest, DataTransferResponse, GetVariablesRequest,
    GetVariablesResponse, LocalListVersionResponse, RemoteStartRequest, RemoteStopRequest,
    ResetRequest, SendLocalListRequest, SendLocalListResponse, SetChargingProfileRequest,
    SetVariablesRequest, SetVariablesResponse, TriggerMessageRequest, UnlockConnectorRequest,
    VariableResultDto, SetVariableStatusDto,
};
use crate::application::events::{
    Event, SharedEventBus, TransactionBilledEvent, TransactionStoppedEvent,
};
use crate::application::charging::commands::dispatcher::ClearChargingProfileCriteria;
use crate::application::ChargePointService;
use crate::application::SharedSessionRegistry;
use crate::application::{
    Availability, ResetKind, SharedCommandDispatcher, TriggerType,
};
use crate::application::BillingService;
use crate::domain::{ChargingLimitType, RepositoryProvider};
use crate::interfaces::http::common::ApiResponse;

/// Command handler state
#[derive(Clone)]
pub struct CommandAppState {
    pub repos: Arc<dyn RepositoryProvider>,
    pub session_registry: SharedSessionRegistry,
    pub command_dispatcher: SharedCommandDispatcher,
    pub event_bus: SharedEventBus,
    pub charge_point_service: Arc<ChargePointService>,
    pub billing_service: Arc<BillingService>,
}

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/remote-start",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = RemoteStartRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn remote_start(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<RemoteStartRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    match state
        .command_dispatcher
        .remote_start(
            &charge_point_id,
            &request.id_tag,
            request.connector_id,
        )
        .await
    {
        Ok(status_str) => {
            let accepted = status_str.contains("Accepted");

            if accepted {
                if let (Some(limit_type_str), Some(limit_value)) =
                    (&request.limit_type, request.limit_value)
                {
                    if let Some(limit_type) = ChargingLimitType::from_str(limit_type_str) {
                        let connector_id = request.connector_id.unwrap_or(1);
                        state.charge_point_service.set_pending_limit(
                            &charge_point_id,
                            connector_id,
                            limit_type,
                            limit_value,
                        );
                        info!(
                            "Set pending charging limit for {}:{} - {} = {}",
                            charge_point_id, connector_id, limit_type_str, limit_value
                        );
                    }
                }
            }

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

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/remote-stop",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = RemoteStopRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn remote_stop(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<RemoteStopRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    match state
        .command_dispatcher
        .remote_stop(
            &charge_point_id,
            request.transaction_id,
        )
        .await
    {
        Ok(status_str) => {
            let accepted = status_str.contains("Accepted");

            if accepted {
                let transaction_id = request.transaction_id;
                match state.repos.transactions().find_by_id(transaction_id).await {
                    Ok(Some(mut tx)) if tx.is_active() => {
                        let meter_stop = tx.meter_stop.unwrap_or(tx.meter_start);
                        tx.stop(meter_stop, Some("RemoteStop".to_string()));

                        if let Err(e) = state.repos.transactions().update(tx.clone()).await {
                            error!(
                                "[{}] Failed to update transaction {} after RemoteStop: {}",
                                charge_point_id, transaction_id, e
                            );
                        } else {
                            info!(
                                "[{}] Transaction {} stopped proactively after RemoteStop accepted",
                                charge_point_id, transaction_id
                            );

                            let energy_wh = tx.energy_consumed().unwrap_or(0);

                            // Calculate billing for the stopped transaction
                            let (total_cost, currency) = match state
                                .billing_service
                                .calculate_transaction_billing(transaction_id, None)
                                .await
                            {
                                Ok(billing) => {
                                    info!(
                                        "[{}] Billing calculated for transaction {}: {} {}",
                                        charge_point_id,
                                        transaction_id,
                                        billing.total_cost as f64 / 100.0,
                                        billing.currency
                                    );

                                    // Publish billing event
                                    state.event_bus.publish(Event::TransactionBilled(
                                        TransactionBilledEvent {
                                            charge_point_id: charge_point_id.clone(),
                                            transaction_id,
                                            energy_kwh: energy_wh as f64 / 1000.0,
                                            duration_minutes: billing.duration_seconds as f64
                                                / 60.0,
                                            energy_cost: billing.energy_cost as f64 / 100.0,
                                            time_cost: billing.time_cost as f64 / 100.0,
                                            session_fee: billing.session_fee as f64 / 100.0,
                                            total_cost: billing.total_cost as f64 / 100.0,
                                            currency: billing.currency.clone(),
                                            tariff_name: None,
                                            timestamp: Utc::now(),
                                        },
                                    ));

                                    (
                                        billing.total_cost as f64 / 100.0,
                                        billing.currency,
                                    )
                                }
                                Err(e) => {
                                    warn!(
                                        "[{}] Billing failed for transaction {}: {}",
                                        charge_point_id, transaction_id, e
                                    );
                                    (0.0, "UZS".to_string())
                                }
                            };

                            state.event_bus.publish(Event::TransactionStopped(
                                TransactionStoppedEvent {
                                    charge_point_id: charge_point_id.clone(),
                                    transaction_id,
                                    id_tag: Some(tx.id_tag.clone()),
                                    meter_stop,
                                    energy_consumed_kwh: energy_wh as f64 / 1000.0,
                                    total_cost,
                                    currency,
                                    reason: Some("RemoteStop".to_string()),
                                    timestamp: Utc::now(),
                                },
                            ));
                        }
                    }
                    Ok(Some(_)) => {
                        warn!(
                            "[{}] Transaction {} already stopped",
                            charge_point_id, transaction_id
                        );
                    }
                    Ok(None) => {
                        warn!(
                            "[{}] Transaction {} not found for proactive stop",
                            charge_point_id, transaction_id
                        );
                    }
                    Err(e) => {
                        error!(
                            "[{}] Failed to get transaction {}: {}",
                            charge_point_id, transaction_id, e
                        );
                    }
                }
            }

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

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/reset",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = ResetRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn reset_charge_point(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<ResetRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_registry.is_connected(&charge_point_id) {
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

    match state
        .command_dispatcher
        .reset(&charge_point_id, reset_type)
        .await
    {
        Ok(status_str) => {
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

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/unlock-connector",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = UnlockConnectorRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn unlock(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<UnlockConnectorRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    match state
        .command_dispatcher
        .unlock_connector(
            &charge_point_id,
            request.connector_id,
        )
        .await
    {
        Ok(status_str) => {
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

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/change-availability",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = ChangeAvailabilityRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn change_avail(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<ChangeAvailabilityRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_registry.is_connected(&charge_point_id) {
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

    match state
        .command_dispatcher
        .change_availability(
            &charge_point_id,
            request.connector_id,
            availability,
        )
        .await
    {
        Ok(status_str) => {
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

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/trigger-message",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = TriggerMessageRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 400, description = "Unknown message type"),
        (status = 404, description = "Not connected")
    )
)]
pub async fn trigger_msg(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<TriggerMessageRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_registry.is_connected(&charge_point_id) {
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

    match state
        .command_dispatcher
        .trigger_message(
            &charge_point_id,
            message_type,
            request.connector_id,
        )
        .await
    {
        Ok(status_str) => {
            let accepted = status_str.contains("Accepted");
            let not_implemented = status_str.contains("NotImplemented");
            Ok(Json(ApiResponse::success(CommandResponse {
                status: status_str,
                message: if accepted {
                    Some("Trigger message accepted".to_string())
                } else if not_implemented {
                    Some("Not implemented by charge point".to_string())
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

/// Query params for GetConfiguration
#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct ConfigurationParams {
    pub keys: Option<String>,
}

/// Single configuration key-value
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ConfigValue {
    pub key: String,
    pub value: Option<String>,
    pub readonly: bool,
}

/// GetConfiguration response
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ConfigurationResponse {
    pub configuration: Vec<ConfigValue>,
    pub unknown_keys: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/configuration",
    tag = "Commands",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID"),
        ("keys" = Option<String>, Query, description = "Comma-separated keys")
    ),
    responses(
        (status = 200, description = "Configuration", body = ApiResponse<ConfigurationResponse>),
        (status = 404, description = "Not connected")
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
    if !state.session_registry.is_connected(&charge_point_id) {
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

    match state
        .command_dispatcher
        .get_configuration(&charge_point_id, keys)
        .await
    {
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

#[utoipa::path(
    put,
    path = "/api/v1/charge-points/{charge_point_id}/configuration",
    tag = "Commands",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    request_body = ChangeConfigurationRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn change_config(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<ChangeConfigurationRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    match state
        .command_dispatcher
        .change_configuration(
            &charge_point_id,
            request.key.clone(),
            request.value.clone(),
        )
        .await
    {
        Ok(status_str) => Ok(Json(ApiResponse::success(CommandResponse {
            status: status_str,
            message: Some(format!("Configuration '{}' update processed", request.key)),
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/local-list-version",
    tag = "Commands",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    responses(
        (status = 200, description = "Local list version", body = ApiResponse<LocalListVersionResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn get_local_list_ver(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
) -> Result<
    Json<ApiResponse<LocalListVersionResponse>>,
    (StatusCode, Json<ApiResponse<LocalListVersionResponse>>),
> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    match state
        .command_dispatcher
        .get_local_list_version(&charge_point_id)
        .await
    {
        Ok(version) => Ok(Json(ApiResponse::success(LocalListVersionResponse {
            list_version: version,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/local-list",
    tag = "Commands",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    request_body = SendLocalListRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<SendLocalListResponse>),
        (status = 404, description = "Not connected"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn send_local_list(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<SendLocalListRequest>,
) -> Result<
    Json<ApiResponse<SendLocalListResponse>>,
    (StatusCode, Json<ApiResponse<SendLocalListResponse>>),
> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    // Validate update_type
    let update_type = request.update_type.to_lowercase();
    if update_type != "full" && update_type != "differential" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "update_type must be 'Full' or 'Differential'",
            )),
        ));
    }

    // Convert DTO entries to domain LocalAuthEntry
    let entries = request.local_authorization_list.map(|list| {
        list.into_iter()
            .map(|e| crate::application::charging::commands::LocalAuthEntry {
                id_tag: e.id_tag,
                status: e.status,
                expiry_date: e.expiry_date,
                parent_id_tag: e.parent_id_tag,
            })
            .collect()
    });

    match state
        .command_dispatcher
        .send_local_list(
            &charge_point_id,
            request.list_version,
            &request.update_type,
            entries,
        )
        .await
    {
        Ok(status_str) => {
            let accepted = status_str.contains("Accepted");
            Ok(Json(ApiResponse::success(SendLocalListResponse {
                status: status_str,
                message: if accepted {
                    Some("Local authorization list updated".to_string())
                } else {
                    Some("Local list update failed or not supported".to_string())
                },
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/clear-cache",
    tag = "Commands",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn clear_auth_cache(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    match state
        .command_dispatcher
        .clear_cache(&charge_point_id)
        .await
    {
        Ok(status_str) => Ok(Json(ApiResponse::success(CommandResponse {
            status: status_str,
            message: Some("Authorization cache cleared".to_string()),
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/data-transfer",
    tag = "Commands",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    request_body = DataTransferRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<DataTransferResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn data_transfer_handler(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<DataTransferRequest>,
) -> Result<
    Json<ApiResponse<DataTransferResponse>>,
    (StatusCode, Json<ApiResponse<DataTransferResponse>>),
> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    match state
        .command_dispatcher
        .data_transfer(
            &charge_point_id,
            request.vendor_id,
            request.message_id,
            request.data,
        )
        .await
    {
        Ok(result) => Ok(Json(ApiResponse::success(DataTransferResponse {
            status: result.status,
            data: result.data,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

// ── v2.0.1-specific command handlers ───────────────────────────────

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/variables/get",
    tag = "Commands (v2.0.1)",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = GetVariablesRequest,
    responses(
        (status = 200, description = "Variables", body = ApiResponse<GetVariablesResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn get_variables(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<GetVariablesRequest>,
) -> Result<
    Json<ApiResponse<GetVariablesResponse>>,
    (StatusCode, Json<ApiResponse<GetVariablesResponse>>),
> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    let variables: Vec<(String, String)> = request
        .variables
        .into_iter()
        .map(|v| (v.component, v.variable))
        .collect();

    match state
        .command_dispatcher
        .get_variables(&charge_point_id, variables)
        .await
    {
        Ok(result) => {
            let results = result
                .results
                .into_iter()
                .map(|r| VariableResultDto {
                    component: r.component,
                    variable: r.variable,
                    status: r.attribute_status,
                    value: r.attribute_value,
                })
                .collect();
            Ok(Json(ApiResponse::success(GetVariablesResponse { results })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/variables/set",
    tag = "Commands (v2.0.1)",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = SetVariablesRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<SetVariablesResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn set_variables(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<SetVariablesRequest>,
) -> Result<
    Json<ApiResponse<SetVariablesResponse>>,
    (StatusCode, Json<ApiResponse<SetVariablesResponse>>),
> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    let variables: Vec<(String, String, String)> = request
        .variables
        .into_iter()
        .map(|v| (v.component, v.variable, v.value))
        .collect();

    match state
        .command_dispatcher
        .set_variables(&charge_point_id, variables)
        .await
    {
        Ok(result) => {
            let results = result
                .results
                .into_iter()
                .map(|r| SetVariableStatusDto {
                    component: r.component,
                    variable: r.variable,
                    status: r.status,
                })
                .collect();
            Ok(Json(ApiResponse::success(SetVariablesResponse { results })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/charging-profile/clear",
    tag = "Commands (v2.0.1)",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = ClearChargingProfileRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn clear_charging_profile(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<ClearChargingProfileRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    let criteria = ClearChargingProfileCriteria {
        charging_profile_id: request.charging_profile_id,
        evse_id: request.evse_id,
        charging_profile_purpose: request.charging_profile_purpose,
        stack_level: request.stack_level,
    };

    match state
        .command_dispatcher
        .clear_charging_profile(&charge_point_id, criteria)
        .await
    {
        Ok(status_str) => {
            let accepted = status_str.contains("Accepted");
            Ok(Json(ApiResponse::success(CommandResponse {
                status: status_str,
                message: if accepted {
                    Some("Charging profile(s) cleared".to_string())
                } else {
                    Some("No matching charging profiles found".to_string())
                },
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/charging-profile/set",
    tag = "Commands (v2.0.1)",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = SetChargingProfileRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn set_charging_profile(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<SetChargingProfileRequest>,
) -> Result<Json<ApiResponse<CommandResponse>>, (StatusCode, Json<ApiResponse<CommandResponse>>)> {
    if !state.session_registry.is_connected(&charge_point_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "Charge point '{}' is not connected",
                charge_point_id
            ))),
        ));
    }

    // Deserialize the raw JSON into the typed ChargingProfileType
    let charging_profile: rust_ocpp::v2_0_1::datatypes::charging_profile_type::ChargingProfileType =
        serde_json::from_value(request.charging_profile).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(format!(
                    "Invalid ChargingProfile JSON: {}",
                    e
                ))),
            )
        })?;

    match state
        .command_dispatcher
        .set_charging_profile(
            &charge_point_id,
            request.evse_id,
            charging_profile,
        )
        .await
    {
        Ok(status_str) => {
            let accepted = status_str.contains("Accepted");
            Ok(Json(ApiResponse::success(CommandResponse {
                status: status_str,
                message: if accepted {
                    Some("Charging profile set".to_string())
                } else {
                    Some("Charging profile rejected by station".to_string())
                },
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}
