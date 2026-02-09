//! OCPP 1.6 message handler

use std::sync::Arc;

use log::{error, info, warn};

use ocpp_rs::v16::call::Call;
use ocpp_rs::v16::call_error::CallError;
use ocpp_rs::v16::call_result::CallResult;
use ocpp_rs::v16::parse::{self, Message};

use crate::application::commands::CommandSender;
use crate::application::handlers::ocpp::action_matcher;
use crate::application::services::{BillingService, ChargePointService};
use crate::notifications::SharedEventBus;

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

    /// Handle an incoming OCPP message
    pub async fn handle(&self, text: &str) -> Option<String> {
        info!("[{}] Received raw message: {}", self.charge_point_id, text);
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
                    "[{}] Standard parser failed: {:?}, trying fallback sanitizer...",
                    self.charge_point_id, e
                );
                // Try to sanitize and re-parse
                match Self::sanitize_and_parse(text) {
                    Some(Message::Call(call)) => {
                        info!(
                            "[{}] Fallback parser succeeded for {:?}",
                            self.charge_point_id,
                            call.payload.as_ref()
                        );
                        self.handle_call(call).await
                    }
                    Some(Message::CallResult(result)) => {
                        info!(
                            "[{}] Fallback parser succeeded for CallResult {}",
                            self.charge_point_id, result.unique_id
                        );
                        self.handle_call_result(result).await;
                        None
                    }
                    Some(Message::CallError(error)) => {
                        self.handle_call_error(error).await;
                        None
                    }
                    None => {
                        error!(
                            "[{}] Failed to parse OCPP message even after sanitization: {:?} | raw: {}",
                            self.charge_point_id, e, text
                        );
                        None
                    }
                }
            }
        }
    }

    /// Sanitize malformed OCPP JSON and attempt re-parse.
    ///
    /// Some charge points send non-compliant messages, e.g.
    /// `"transactionId": null` in StopTransaction (should be i32).
    /// This method fixes known issues before re-parsing.
    fn sanitize_and_parse(text: &str) -> Option<Message> {
        let mut value: serde_json::Value = serde_json::from_str(text).ok()?;

        let arr = value.as_array_mut()?;
        let msg_type = arr.first()?.as_u64()?;

        // Sanitize CallResult (type 3): must be [3, uniqueId, {payload}]
        if msg_type == 3 {
            // Minimum: [3, "id"] → pad with empty payload {}
            while arr.len() < 3 {
                arr.push(serde_json::json!({}));
            }
            // If payload is null, replace with {}
            if arr.get(2).map_or(false, |v| v.is_null()) {
                arr[2] = serde_json::json!({});
                warn!("Sanitizer: CallResult payload was null, replaced with {{}}");
            }
        }

        // Sanitize CallError (type 4): must be [4, uniqueId, errorCode, errorDescription, {errorDetails}]
        // Some stations send truncated errors like [4,"CS-3"] or [4,"CS-3","NotImplemented"]
        if msg_type == 4 {
            let unique_id = arr.get(1).cloned().unwrap_or(serde_json::json!("unknown"));
            while arr.len() < 5 {
                match arr.len() {
                    2 => {
                        warn!("Sanitizer: CallError missing errorCode for {}, defaulting to NotImplemented", unique_id);
                        arr.push(serde_json::json!("NotImplemented"));
                    }
                    3 => {
                        arr.push(serde_json::json!(""));
                    }
                    4 => {
                        arr.push(serde_json::json!({}));
                    }
                    _ => break,
                }
            }
        }

        // Sanitize Call messages (type 2): fix null values in required i32/u32 fields
        if msg_type == 2 && arr.len() >= 4 {
            let action = arr.get(2)?.as_str()?.to_string();
            let payload = arr.get_mut(3)?;

            if let Some(obj) = payload.as_object_mut() {
                match action.as_str() {
                    "StopTransaction" => {
                        // transactionId: null → 0
                        if let Some(v) = obj.get("transactionId") {
                            if v.is_null() {
                                warn!("Sanitizer: StopTransaction.transactionId was null, setting to 0");
                                obj.insert(
                                    "transactionId".to_string(),
                                    serde_json::Value::Number(0.into()),
                                );
                            }
                        }
                        // meterStop: null → 0
                        if let Some(v) = obj.get("meterStop") {
                            if v.is_null() {
                                warn!("Sanitizer: StopTransaction.meterStop was null, setting to 0");
                                obj.insert(
                                    "meterStop".to_string(),
                                    serde_json::Value::Number(0.into()),
                                );
                            }
                        }
                    }
                    "StartTransaction" => {
                        // meterStart: null → 0
                        if let Some(v) = obj.get("meterStart") {
                            if v.is_null() {
                                warn!("Sanitizer: StartTransaction.meterStart was null, setting to 0");
                                obj.insert(
                                    "meterStart".to_string(),
                                    serde_json::Value::Number(0.into()),
                                );
                            }
                        }
                        // connectorId: null → 0
                        if let Some(v) = obj.get("connectorId") {
                            if v.is_null() {
                                warn!("Sanitizer: StartTransaction.connectorId was null, setting to 0");
                                obj.insert(
                                    "connectorId".to_string(),
                                    serde_json::Value::Number(0.into()),
                                );
                            }
                        }
                    }
                    "MeterValues" => {
                        // connectorId: null → 0
                        if let Some(v) = obj.get("connectorId") {
                            if v.is_null() {
                                warn!("Sanitizer: MeterValues.connectorId was null, setting to 0");
                                obj.insert(
                                    "connectorId".to_string(),
                                    serde_json::Value::Number(0.into()),
                                );
                            }
                        }
                    }
                    "StatusNotification" => {
                        // connectorId: null → 0
                        if let Some(v) = obj.get("connectorId") {
                            if v.is_null() {
                                warn!("Sanitizer: StatusNotification.connectorId was null, setting to 0");
                                obj.insert(
                                    "connectorId".to_string(),
                                    serde_json::Value::Number(0.into()),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let sanitized = serde_json::to_string(&value).ok()?;
        parse::deserialize_to_message(&sanitized).ok()
    }

    /// Handle Call messages (requests from charge point)
    async fn handle_call(&self, call: Call) -> Option<String> {
        let message_id = call.unique_id.clone();

        info!(
            "[{}] Received {:?}",
            self.charge_point_id,
            call.payload.as_ref()
        );

        let result_payload = action_matcher(self, call.payload).await;
        let response = Message::CallResult(CallResult::new(message_id, result_payload));

        match parse::serialize_message(&response) {
            Ok(json) => Some(json),
            Err(e) => {
                error!(
                    "[{}] Failed to serialize response: {:?}",
                    self.charge_point_id, e
                );
                None
            }
        }
    }

    /// Handle CallResult messages (responses to commands we sent)
    async fn handle_call_result(&self, result: CallResult) {
        info!(
            "[{}] Received CallResult for message: {}",
            self.charge_point_id, result.unique_id
        );
        self.command_sender
            .handle_response(&self.charge_point_id, &result.unique_id, result.payload);
    }

    /// Handle CallError messages (error responses to commands we sent)
    async fn handle_call_error(&self, error: CallError) {
        warn!(
            "[{}] Received CallError for message {}: {:?}",
            self.charge_point_id, error.unique_id, error.error_code
        );
        self.command_sender.handle_error(
            &self.charge_point_id,
            &error.unique_id,
            &error.error_code,
            &error.error_description,
        );
    }
}
