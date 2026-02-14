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
    ClearMonitoringResultDto, ClearVariableMonitoringRequest, ClearVariableMonitoringResponse,
    ChargingProfileDto, ChargingProfileListResponse,
    CommandResponse, DataTransferRequest, DataTransferResponse, GetBaseReportRequest,
    GetBaseReportResponse, GetChargingProfilesHttpRequest, GetChargingProfilesHttpResponse,
    GetCompositeScheduleRequest,
    GetCompositeScheduleResponse, GetDiagnosticsRequest, GetDiagnosticsResponse,
    GetTransactionStatusRequest, GetTransactionStatusResponse,
    GetVariablesRequest, GetVariablesResponse,
    LocalListVersionResponse, MonitoringResultDto,
    RemoteStartRequest, RemoteStopRequest, ResetRequest,
    SendLocalListRequest, SendLocalListResponse, SetChargingProfileRequest,
    SetMonitoringBaseRequest, SetMonitoringBaseResponse,
    SetVariableMonitoringRequest, SetVariableMonitoringResponse,
    SetVariablesRequest, SetVariablesResponse,
    TriggerMessageRequest, UnlockConnectorRequest,
    UpdateFirmwareRequest, UpdateFirmwareResponse, VariableResultDto,
    SetVariableStatusDto,
};
use crate::application::events::{
    Event, SharedEventBus, TransactionBilledEvent, TransactionStoppedEvent,
};
use crate::application::charging::commands::dispatcher::ClearChargingProfileCriteria;
use crate::application::charging::commands::dispatcher::GetChargingProfilesCriteria;
use crate::application::charging::commands::dispatcher::MonitorDescriptor;
use crate::application::ChargePointService;
use crate::application::SharedSessionRegistry;
use crate::application::{
    Availability, ResetKind, SharedCommandDispatcher, TriggerType,
};
use crate::application::BillingService;
use crate::domain::{ChargingLimitType, RepositoryProvider};
use crate::interfaces::http::common::ApiResponse;

use crate::application::charging::services::device_report::{
    DeviceReport, SharedDeviceReportStore,
};

/// Command handler state
#[derive(Clone)]
pub struct CommandAppState {
    pub repos: Arc<dyn RepositoryProvider>,
    pub session_registry: SharedSessionRegistry,
    pub command_dispatcher: SharedCommandDispatcher,
    pub event_bus: SharedEventBus,
    pub charge_point_service: Arc<ChargePointService>,
    pub billing_service: Arc<BillingService>,
    pub report_store: SharedDeviceReportStore,
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
                            request.external_order_id.clone(),
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
                                    external_order_id: tx.external_order_id.clone(),
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
    path = "/api/v1/charge-points/{charge_point_id}/composite-schedule",
    tag = "Commands",
    security(("bearer_auth" = []), ("api_key" = [])),
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    request_body = GetCompositeScheduleRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<GetCompositeScheduleResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn get_composite_schedule(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<GetCompositeScheduleRequest>,
) -> Result<
    Json<ApiResponse<GetCompositeScheduleResponse>>,
    (StatusCode, Json<ApiResponse<GetCompositeScheduleResponse>>),
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
        .get_composite_schedule(
            &charge_point_id,
            request.connector_id,
            request.duration,
            request.charging_rate_unit.as_deref(),
        )
        .await
    {
        Ok(result) => {
            Ok(Json(ApiResponse::success(GetCompositeScheduleResponse {
                status: result.status,
                schedule: result.schedule,
                connector_id: result.connector_id,
                schedule_start: result.schedule_start,
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
    tag = "Commands",
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
        charging_profile_purpose: request.charging_profile_purpose.clone(),
        stack_level: request.stack_level,
    };

    match state
        .command_dispatcher
        .clear_charging_profile(&charge_point_id, criteria)
        .await
    {
        Ok(status_str) => {
            let accepted = status_str.contains("Accepted");

            // Persist deactivation in DB when the station accepts
            if accepted {
                if let Some(profile_id) = request.charging_profile_id {
                    if let Err(e) = state
                        .repos
                        .charging_profiles()
                        .deactivate_by_profile_id(&charge_point_id, profile_id)
                        .await
                    {
                        warn!("Failed to deactivate profile {} in DB: {}", profile_id, e);
                    }
                } else {
                    if let Err(e) = state
                        .repos
                        .charging_profiles()
                        .deactivate_by_criteria(
                            &charge_point_id,
                            request.evse_id,
                            request.charging_profile_purpose.as_deref(),
                            request.stack_level,
                        )
                        .await
                    {
                        warn!("Failed to deactivate profiles by criteria in DB: {}", e);
                    }
                }
            }

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
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = SetChargingProfileRequest,
    responses(
        (status = 200, description = "Result", body = ApiResponse<CommandResponse>),
        (status = 404, description = "Not connected"),
        (status = 400, description = "Invalid charging profile JSON")
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

    match state
        .command_dispatcher
        .set_charging_profile(
            &charge_point_id,
            request.evse_id,
            request.charging_profile.clone(),
        )
        .await
    {
        Ok(status_str) => {
            let accepted = status_str.contains("Accepted");

            // Persist the profile in DB when accepted
            if accepted {
                let profile_json = &request.charging_profile;
                let profile_id = profile_json
                    .get("chargingProfileId")
                    .or_else(|| profile_json.get("id"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let stack_level = profile_json
                    .get("stackLevel")
                    .or_else(|| profile_json.get("stack_level"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let purpose = profile_json
                    .get("chargingProfilePurpose")
                    .or_else(|| profile_json.get("charging_profile_purpose"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("TxDefaultProfile")
                    .to_string();
                let kind = profile_json
                    .get("chargingProfileKind")
                    .or_else(|| profile_json.get("charging_profile_kind"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Absolute")
                    .to_string();
                let recurrency_kind = profile_json
                    .get("recurrencyKind")
                    .or_else(|| profile_json.get("recurrency_kind"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let valid_from = profile_json
                    .get("validFrom")
                    .or_else(|| profile_json.get("valid_from"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));
                let valid_to = profile_json
                    .get("validTo")
                    .or_else(|| profile_json.get("valid_to"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                // Extract schedule(s) as JSON
                let schedule_json = profile_json
                    .get("chargingSchedule")
                    .or_else(|| profile_json.get("charging_schedule"))
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "[]".to_string());

                let now = Utc::now();
                let domain_profile = crate::domain::ChargingProfile {
                    id: 0, // auto-generated
                    charge_point_id: charge_point_id.clone(),
                    evse_id: request.evse_id,
                    profile_id,
                    stack_level,
                    purpose,
                    kind,
                    recurrency_kind,
                    valid_from,
                    valid_to,
                    schedule_json,
                    is_active: true,
                    created_at: now,
                    updated_at: now,
                };

                if let Err(e) = state.repos.charging_profiles().save(domain_profile).await {
                    warn!("Failed to save charging profile to DB: {}", e);
                }
            }

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

// ─── Firmware Management ───────────────────────────────────────────────────

/// Instruct a charge point to download and install firmware.
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/firmware/update",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    request_body = UpdateFirmwareRequest,
    responses(
        (status = 200, description = "Firmware update accepted", body = ApiResponse<UpdateFirmwareResponse>),
    )
)]
pub async fn update_firmware(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<UpdateFirmwareRequest>,
) -> Result<
    Json<ApiResponse<UpdateFirmwareResponse>>,
    (StatusCode, Json<ApiResponse<UpdateFirmwareResponse>>),
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

    let retrieve_date = match chrono::DateTime::parse_from_rfc3339(&request.retrieve_date) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(format!(
                    "Invalid retrieve_date: {}",
                    e
                ))),
            ));
        }
    };

    match state
        .command_dispatcher
        .update_firmware(
            &charge_point_id,
            &request.location,
            retrieve_date,
            request.retries,
            request.retry_interval,
        )
        .await
    {
        Ok(status) => Ok(Json(ApiResponse::success(UpdateFirmwareResponse {
            status,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Request diagnostics or log upload from a charge point.
///
/// v1.6: sends GetDiagnostics. v2.0.1: sends GetLog.
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/diagnostics",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    request_body = GetDiagnosticsRequest,
    responses(
        (status = 200, description = "Diagnostics request accepted", body = ApiResponse<GetDiagnosticsResponse>),
    )
)]
pub async fn get_diagnostics(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<GetDiagnosticsRequest>,
) -> Result<
    Json<ApiResponse<GetDiagnosticsResponse>>,
    (StatusCode, Json<ApiResponse<GetDiagnosticsResponse>>),
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

    let start_time = match &request.start_time {
        Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
            Ok(dt) => Some(dt.with_timezone(&chrono::Utc)),
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(format!("Invalid start_time: {}", e))),
                ));
            }
        },
        None => None,
    };

    let stop_time = match &request.stop_time {
        Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
            Ok(dt) => Some(dt.with_timezone(&chrono::Utc)),
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(format!("Invalid stop_time: {}", e))),
                ));
            }
        },
        None => None,
    };

    match state
        .command_dispatcher
        .get_diagnostics(
            &charge_point_id,
            &request.location,
            request.retries,
            request.retry_interval,
            start_time,
            stop_time,
            request.log_type.as_deref(),
        )
        .await
    {
        Ok(result) => Ok(Json(ApiResponse::success(GetDiagnosticsResponse {
            status: result.status,
            filename: result.filename,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

// ─── Device Reports ────────────────────────────────────────────────────

/// Request a base report from a v2.0.1 charge point.
///
/// The charge point will asynchronously send NotifyReport messages.
/// Use GET /charge-points/{id}/report to retrieve the assembled report.
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/report",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    request_body = GetBaseReportRequest,
    responses(
        (status = 200, description = "Report request accepted", body = ApiResponse<GetBaseReportResponse>),
    )
)]
pub async fn request_base_report(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<GetBaseReportRequest>,
) -> Result<
    Json<ApiResponse<GetBaseReportResponse>>,
    (StatusCode, Json<ApiResponse<GetBaseReportResponse>>),
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

    // Generate a unique request_id
    let request_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32;

    // Initialize the report store slot before sending
    state
        .report_store
        .init_report(&charge_point_id, request_id);

    match state
        .command_dispatcher
        .get_base_report(&charge_point_id, request_id, &request.report_base)
        .await
    {
        Ok(result) => Ok(Json(ApiResponse::success(GetBaseReportResponse {
            status: result.status,
            request_id,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Retrieve the latest device report for a charge point.
///
/// Reports are assembled from NotifyReport messages received after a GetBaseReport command.
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/report",
    tag = "Commands",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID"),
        ("request_id" = Option<i32>, Query, description = "Specific report request_id (optional, defaults to latest)"),
    ),
    responses(
        (status = 200, description = "Device report", body = ApiResponse<DeviceReport>),
    )
)]
pub async fn get_device_report(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Query(params): Query<ReportQueryParams>,
) -> Result<
    Json<ApiResponse<crate::application::charging::services::device_report::DeviceReport>>,
    (
        StatusCode,
        Json<ApiResponse<crate::application::charging::services::device_report::DeviceReport>>,
    ),
> {
    let report = match params.request_id {
        Some(rid) => state.report_store.get_report(&charge_point_id, rid),
        None => state.report_store.get_latest_report(&charge_point_id),
    };

    match report {
        Some(r) => Ok(Json(ApiResponse::success(r))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!(
                "No device report found for charge point '{}'",
                charge_point_id
            ))),
        )),
    }
}

/// Query parameters for report retrieval.
#[derive(Debug, serde::Deserialize)]
pub struct ReportQueryParams {
    pub request_id: Option<i32>,
}

// ─── Variable Monitoring (v2.0.1 only) ────────────────────────────────

/// Configure variable monitors on a charge point (v2.0.1 only).
///
/// Sends a SetVariableMonitoring command with one or more monitor descriptors.
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/monitoring/set",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = SetVariableMonitoringRequest,
    responses(
        (status = 200, description = "Monitoring set results", body = ApiResponse<SetVariableMonitoringResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn set_variable_monitoring_handler(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<SetVariableMonitoringRequest>,
) -> Result<
    Json<ApiResponse<SetVariableMonitoringResponse>>,
    (StatusCode, Json<ApiResponse<SetVariableMonitoringResponse>>),
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

    let monitors: Vec<MonitorDescriptor> = request
        .monitors
        .into_iter()
        .map(|m| MonitorDescriptor {
            component: m.component,
            variable: m.variable,
            monitor_type: m.monitor_type,
            value: m.value,
            severity: m.severity,
            transaction: m.transaction,
            id: m.id,
        })
        .collect();

    match state
        .command_dispatcher
        .set_variable_monitoring(&charge_point_id, monitors)
        .await
    {
        Ok(result) => {
            let results = result
                .results
                .into_iter()
                .map(|r| MonitoringResultDto {
                    component: r.component,
                    variable: r.variable,
                    status: r.status,
                    monitor_id: r.monitor_id,
                    monitor_type: r.monitor_type,
                })
                .collect();
            Ok(Json(ApiResponse::success(SetVariableMonitoringResponse {
                results,
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Set the monitoring base level on a charge point (v2.0.1 only).
///
/// Controls which monitors are active: All, FactoryDefault, or HardWiredOnly.
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/monitoring/base",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = SetMonitoringBaseRequest,
    responses(
        (status = 200, description = "Monitoring base result", body = ApiResponse<SetMonitoringBaseResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn set_monitoring_base_handler(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<SetMonitoringBaseRequest>,
) -> Result<
    Json<ApiResponse<SetMonitoringBaseResponse>>,
    (StatusCode, Json<ApiResponse<SetMonitoringBaseResponse>>),
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
        .set_monitoring_base(&charge_point_id, &request.monitoring_base)
        .await
    {
        Ok(result) => Ok(Json(ApiResponse::success(SetMonitoringBaseResponse {
            status: result.status,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// Clear variable monitors on a charge point (v2.0.1 only).
///
/// Removes monitors by their IDs.
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/monitoring/clear",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = ClearVariableMonitoringRequest,
    responses(
        (status = 200, description = "Clear monitoring results", body = ApiResponse<ClearVariableMonitoringResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn clear_variable_monitoring_handler(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<ClearVariableMonitoringRequest>,
) -> Result<
    Json<ApiResponse<ClearVariableMonitoringResponse>>,
    (StatusCode, Json<ApiResponse<ClearVariableMonitoringResponse>>),
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
        .clear_variable_monitoring(&charge_point_id, request.ids)
        .await
    {
        Ok(result) => {
            let results = result
                .results
                .into_iter()
                .map(|r| ClearMonitoringResultDto {
                    id: r.id,
                    status: r.status,
                })
                .collect();
            Ok(Json(ApiResponse::success(ClearVariableMonitoringResponse {
                results,
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

// ─── Charging Profile Management ───────────────────────────────────────────

/// Request the charge point to report its installed charging profiles (v2.0.1 only).
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/charging-profiles/request",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = GetChargingProfilesHttpRequest,
    responses(
        (status = 200, description = "GetChargingProfiles result", body = ApiResponse<GetChargingProfilesHttpResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn get_charging_profiles_handler(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<GetChargingProfilesHttpRequest>,
) -> Result<
    Json<ApiResponse<GetChargingProfilesHttpResponse>>,
    (StatusCode, Json<ApiResponse<GetChargingProfilesHttpResponse>>),
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

    let criteria = GetChargingProfilesCriteria {
        evse_id: request.evse_id,
        purpose: request.purpose,
        stack_level: request.stack_level,
        profile_ids: request.profile_ids,
    };

    match state
        .command_dispatcher
        .get_charging_profiles(&charge_point_id, request.request_id, criteria)
        .await
    {
        Ok(result) => Ok(Json(ApiResponse::success(GetChargingProfilesHttpResponse {
            status: result.status,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}

/// List stored charging profiles for a charge point (from DB).
#[utoipa::path(
    get,
    path = "/api/v1/charge-points/{charge_point_id}/charging-profiles",
    tag = "Commands",
    params(
        ("charge_point_id" = String, Path, description = "Charge point ID"),
        ("active_only" = Option<bool>, Query, description = "Return only active profiles (default: true)")
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "Stored charging profiles", body = ApiResponse<ChargingProfileListResponse>),
        (status = 500, description = "Database error")
    )
)]
pub async fn list_charging_profiles(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Query(params): Query<ChargingProfileQueryParams>,
) -> Result<
    Json<ApiResponse<ChargingProfileListResponse>>,
    (StatusCode, Json<ApiResponse<ChargingProfileListResponse>>),
> {
    let active_only = params.active_only.unwrap_or(true);

    let profiles_result = if active_only {
        state
            .repos
            .charging_profiles()
            .find_active_for_charge_point(&charge_point_id)
            .await
    } else {
        state
            .repos
            .charging_profiles()
            .find_all_for_charge_point(&charge_point_id)
            .await
    };

    match profiles_result {
        Ok(profiles) => {
            let dtos: Vec<ChargingProfileDto> = profiles
                .into_iter()
                .map(|p| ChargingProfileDto {
                    id: p.id,
                    charge_point_id: p.charge_point_id,
                    evse_id: p.evse_id,
                    profile_id: p.profile_id,
                    stack_level: p.stack_level,
                    purpose: p.purpose,
                    kind: p.kind,
                    recurrency_kind: p.recurrency_kind,
                    valid_from: p.valid_from.map(|dt| dt.to_rfc3339()),
                    valid_to: p.valid_to.map(|dt| dt.to_rfc3339()),
                    schedule_json: p.schedule_json,
                    is_active: p.is_active,
                    created_at: p.created_at.to_rfc3339(),
                    updated_at: p.updated_at.to_rfc3339(),
                })
                .collect();
            Ok(Json(ApiResponse::success(ChargingProfileListResponse {
                profiles: dtos,
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!(
                "Failed to load charging profiles: {}",
                e
            ))),
        )),
    }
}

/// Query params for listing charging profiles.
#[derive(Debug, serde::Deserialize)]
pub struct ChargingProfileQueryParams {
    pub active_only: Option<bool>,
}

// ─── Transaction Status (v2.0.1) ───────────────────────────────────────────

/// Ask the charge point whether a transaction is ongoing and if messages are queued.
#[utoipa::path(
    post,
    path = "/api/v1/charge-points/{charge_point_id}/transaction-status",
    tag = "Commands",
    params(("charge_point_id" = String, Path, description = "Charge point ID")),
    security(("bearer_auth" = []), ("api_key" = [])),
    request_body = GetTransactionStatusRequest,
    responses(
        (status = 200, description = "Transaction status", body = ApiResponse<GetTransactionStatusResponse>),
        (status = 404, description = "Not connected")
    )
)]
pub async fn get_transaction_status(
    State(state): State<CommandAppState>,
    Path(charge_point_id): Path<String>,
    Json(request): Json<GetTransactionStatusRequest>,
) -> Result<
    Json<ApiResponse<GetTransactionStatusResponse>>,
    (StatusCode, Json<ApiResponse<GetTransactionStatusResponse>>),
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
        .get_transaction_status(&charge_point_id, request.transaction_id)
        .await
    {
        Ok(result) => Ok(Json(ApiResponse::success(GetTransactionStatusResponse {
            ongoing_indicator: result.ongoing_indicator,
            messages_in_queue: result.messages_in_queue,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )),
    }
}