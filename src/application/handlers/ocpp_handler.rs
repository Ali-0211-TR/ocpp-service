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
                error!(
                    "[{}] Failed to parse OCPP message: {:?}",
                    self.charge_point_id, e
                );
                None
            }
        }
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
