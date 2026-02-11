//! OCPP 1.6 message handler

use std::sync::Arc;

use ocpp_rs::v16::call::Call;
use ocpp_rs::v16::call_error::CallError;
use ocpp_rs::v16::call_result::CallResult;
use ocpp_rs::v16::parse::{self, Message};
use tracing::{error, info, warn};

use crate::application::commands::CommandSender;
use crate::application::events::SharedEventBus;
use crate::application::handlers::ocpp::action_matcher;
use crate::application::services::{BillingService, ChargePointService};

/// Handler for OCPP 1.6 messages
pub struct OcppHandler {
    pub charge_point_id: String,
    pub service: Arc<ChargePointService>,
    pub billing_service: Arc<BillingService>,
    pub command_sender: Arc<CommandSender>,
    pub event_bus: SharedEventBus,
}

impl OcppHandler {
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
        match parse::deserialize_to_message(text) {
            Ok(Message::Call(call)) => self.handle_call(call).await,
            Ok(Message::CallResult(result)) => {
                self.handle_call_result(result).await;
                None
            }
            Ok(Message::CallError(error)) => {
                self.handle_call_error(error).await;
                None
            }
            Err(e) => {
                warn!(
                    charge_point_id = self.charge_point_id.as_str(),
                    error = ?e,
                    "Standard parser failed, trying fallback sanitizer..."
                );
                match Self::sanitize_and_parse(text) {
                    Some(Message::Call(call)) => {
                        info!(
                            charge_point_id = self.charge_point_id.as_str(),
                            "Fallback parser succeeded"
                        );
                        self.handle_call(call).await
                    }
                    Some(Message::CallResult(result)) => {
                        self.handle_call_result(result).await;
                        None
                    }
                    Some(Message::CallError(error)) => {
                        self.handle_call_error(error).await;
                        None
                    }
                    None => {
                        error!(
                            charge_point_id = self.charge_point_id.as_str(),
                            error = ?e,
                            raw = text,
                            "Failed to parse OCPP message even after sanitization"
                        );
                        None
                    }
                }
            }
        }
    }

    fn sanitize_and_parse(text: &str) -> Option<Message> {
        let mut value: serde_json::Value = serde_json::from_str(text).ok()?;
        let arr = value.as_array_mut()?;
        let msg_type = arr.first()?.as_u64()?;

        if msg_type == 3 {
            while arr.len() < 3 {
                arr.push(serde_json::json!({}));
            }
            if arr.get(2).map_or(false, |v| v.is_null()) {
                arr[2] = serde_json::json!({});
            }
        }

        if msg_type == 4 {
            let unique_id = arr.get(1).cloned().unwrap_or(serde_json::json!("unknown"));
            while arr.len() < 5 {
                match arr.len() {
                    2 => {
                        warn!("Sanitizer: CallError missing errorCode for {}, defaulting to NotImplemented", unique_id);
                        arr.push(serde_json::json!("NotImplemented"));
                    }
                    3 => arr.push(serde_json::json!("")),
                    4 => arr.push(serde_json::json!({})),
                    _ => break,
                }
            }
        }

        if msg_type == 2 && arr.len() >= 4 {
            let action = arr.get(2)?.as_str()?.to_string();
            let payload = arr.get_mut(3)?;

            if let Some(obj) = payload.as_object_mut() {
                match action.as_str() {
                    "StopTransaction" => {
                        if obj.get("transactionId").map_or(false, |v| v.is_null()) {
                            obj.insert("transactionId".to_string(), serde_json::Value::Number(0.into()));
                        }
                        if obj.get("meterStop").map_or(false, |v| v.is_null()) {
                            obj.insert("meterStop".to_string(), serde_json::Value::Number(0.into()));
                        }
                    }
                    "StartTransaction" => {
                        if obj.get("meterStart").map_or(false, |v| v.is_null()) {
                            obj.insert("meterStart".to_string(), serde_json::Value::Number(0.into()));
                        }
                        if obj.get("connectorId").map_or(false, |v| v.is_null()) {
                            obj.insert("connectorId".to_string(), serde_json::Value::Number(0.into()));
                        }
                    }
                    "MeterValues" | "StatusNotification" => {
                        if obj.get("connectorId").map_or(false, |v| v.is_null()) {
                            obj.insert("connectorId".to_string(), serde_json::Value::Number(0.into()));
                        }
                    }
                    _ => {}
                }
            }
        }

        let sanitized = serde_json::to_string(&value).ok()?;
        parse::deserialize_to_message(&sanitized).ok()
    }

    async fn handle_call(&self, call: Call) -> Option<String> {
        let message_id = call.unique_id.clone();

        info!(
            charge_point_id = self.charge_point_id.as_str(),
            action = ?call.payload.as_ref(),
            "Received Call"
        );

        let result_payload = action_matcher(self, call.payload).await;
        let response = Message::CallResult(CallResult::new(message_id, result_payload));

        match parse::serialize_message(&response) {
            Ok(json) => Some(json),
            Err(e) => {
                error!(
                    charge_point_id = self.charge_point_id.as_str(),
                    error = ?e,
                    "Failed to serialize response"
                );
                None
            }
        }
    }

    async fn handle_call_result(&self, result: CallResult) {
        info!(
            charge_point_id = self.charge_point_id.as_str(),
            message_id = result.unique_id.as_str(),
            "Received CallResult"
        );
        self.command_sender
            .handle_response(&self.charge_point_id, &result.unique_id, result.payload);
    }

    async fn handle_call_error(&self, error: CallError) {
        warn!(
            charge_point_id = self.charge_point_id.as_str(),
            message_id = error.unique_id.as_str(),
            error_code = error.error_code.as_str(),
            "Received CallError"
        );
        self.command_sender.handle_error(
            &self.charge_point_id,
            &error.unique_id,
            &error.error_code,
            &error.error_description,
        );
    }
}
