//! OCPP 1.6 message handler
//!
//! Parses raw OCPP-J frames, dispatches to action handlers,
//! and serializes responses using `rust_ocpp::v1_6` types.

use std::sync::Arc;

use serde_json::Value;
use tracing::{error, info, warn};

use crate::application::commands::CommandSender;
use crate::application::events::SharedEventBus;
use crate::application::handlers::ocpp_v16::action_matcher;
use crate::application::services::{BillingService, ChargePointService};
use crate::support::ocpp_frame::OcppFrame;

/// Handler for OCPP 1.6 messages
pub struct OcppHandlerV16 {
    pub charge_point_id: String,
    pub service: Arc<ChargePointService>,
    pub billing_service: Arc<BillingService>,
    pub command_sender: Arc<CommandSender>,
    pub event_bus: SharedEventBus,
}

impl OcppHandlerV16 {
    pub fn new(
        charge_point_id: impl Into<String>,
        service: Arc<ChargePointService>,
        billing_service: Arc<BillingService>,
        command_sender: Arc<CommandSender>,
        event_bus: SharedEventBus,
    ) -> Self {
        Self {
            charge_point_id: charge_point_id.into(),
            service,
            billing_service,
            command_sender,
            event_bus,
        }
    }

    pub async fn handle(&self, text: &str) -> Option<String> {
        info!(
            charge_point_id = self.charge_point_id.as_str(),
            "Received raw message: {}", text
        );

        let frame = match OcppFrame::parse(text) {
            Ok(f) => f,
            Err(e) => {
                warn!(
                    charge_point_id = self.charge_point_id.as_str(),
                    error = %e,
                    "Standard parser failed, trying fallback sanitizer..."
                );
                match Self::sanitize_and_parse(text) {
                    Some(f) => {
                        info!(
                            charge_point_id = self.charge_point_id.as_str(),
                            "Fallback parser succeeded"
                        );
                        f
                    }
                    None => {
                        error!(
                            charge_point_id = self.charge_point_id.as_str(),
                            error = %e,
                            raw = text,
                            "Failed to parse OCPP message even after sanitization"
                        );
                        return None;
                    }
                }
            }
        };

        match frame {
            OcppFrame::Call {
                unique_id,
                action,
                payload,
            } => self.handle_call(&unique_id, &action, payload).await,

            OcppFrame::CallResult { unique_id, payload } => {
                self.handle_call_result(&unique_id, payload).await;
                None
            }

            OcppFrame::CallError {
                unique_id,
                error_code,
                error_description,
                ..
            } => {
                self.handle_call_error(&unique_id, &error_code, &error_description)
                    .await;
                None
            }
        }
    }

    /// Sanitize malformed OCPP-J frames and re-parse.
    fn sanitize_and_parse(text: &str) -> Option<OcppFrame> {
        let mut value: serde_json::Value = serde_json::from_str(text).ok()?;
        let arr = value.as_array_mut()?;
        let msg_type = arr.first()?.as_u64()?;

        // CallResult: ensure at least 3 elements
        if msg_type == 3 {
            while arr.len() < 3 {
                arr.push(serde_json::json!({}));
            }
            if arr.get(2).map_or(false, |v| v.is_null()) {
                arr[2] = serde_json::json!({});
            }
        }

        // CallError: ensure at least 5 elements
        if msg_type == 4 {
            let unique_id = arr.get(1).cloned().unwrap_or(serde_json::json!("unknown"));
            while arr.len() < 5 {
                match arr.len() {
                    2 => {
                        warn!(
                            "Sanitizer: CallError missing errorCode for {}, defaulting to NotImplemented",
                            unique_id
                        );
                        arr.push(serde_json::json!("NotImplemented"));
                    }
                    3 => arr.push(serde_json::json!("")),
                    4 => arr.push(serde_json::json!({})),
                    _ => break,
                }
            }
        }

        // Call: fix common null-field issues
        if msg_type == 2 && arr.len() >= 4 {
            let action = arr.get(2)?.as_str()?.to_string();
            let payload = arr.get_mut(3)?;

            if let Some(obj) = payload.as_object_mut() {
                match action.as_str() {
                    "StopTransaction" => {
                        if obj.get("transactionId").map_or(false, |v| v.is_null()) {
                            obj.insert(
                                "transactionId".to_string(),
                                serde_json::Value::Number(0.into()),
                            );
                        }
                        if obj.get("meterStop").map_or(false, |v| v.is_null()) {
                            obj.insert(
                                "meterStop".to_string(),
                                serde_json::Value::Number(0.into()),
                            );
                        }
                    }
                    "StartTransaction" => {
                        if obj.get("meterStart").map_or(false, |v| v.is_null()) {
                            obj.insert(
                                "meterStart".to_string(),
                                serde_json::Value::Number(0.into()),
                            );
                        }
                        if obj.get("connectorId").map_or(false, |v| v.is_null()) {
                            obj.insert(
                                "connectorId".to_string(),
                                serde_json::Value::Number(0.into()),
                            );
                        }
                    }
                    "MeterValues" | "StatusNotification" => {
                        if obj.get("connectorId").map_or(false, |v| v.is_null()) {
                            obj.insert(
                                "connectorId".to_string(),
                                serde_json::Value::Number(0.into()),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        let sanitized = serde_json::to_string(&value).ok()?;
        OcppFrame::parse(&sanitized).ok()
    }

    async fn handle_call(
        &self,
        unique_id: &str,
        action: &str,
        payload: Value,
    ) -> Option<String> {
        info!(
            charge_point_id = self.charge_point_id.as_str(),
            action,
            "Received Call"
        );

        let response_payload = action_matcher(self, action, &payload).await;

        let response = OcppFrame::CallResult {
            unique_id: unique_id.to_string(),
            payload: response_payload,
        };

        Some(response.serialize())
    }

    async fn handle_call_result(&self, unique_id: &str, payload: Value) {
        info!(
            charge_point_id = self.charge_point_id.as_str(),
            message_id = unique_id,
            "Received CallResult"
        );
        self.command_sender
            .handle_response(&self.charge_point_id, unique_id, payload);
    }

    async fn handle_call_error(
        &self,
        unique_id: &str,
        error_code: &str,
        error_description: &str,
    ) {
        warn!(
            charge_point_id = self.charge_point_id.as_str(),
            message_id = unique_id,
            error_code,
            "Received CallError"
        );
        self.command_sender
            .handle_error(&self.charge_point_id, unique_id, error_code, error_description);
    }
}
