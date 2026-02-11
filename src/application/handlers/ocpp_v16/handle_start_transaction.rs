//! StartTransaction handler

use rust_ocpp::v1_6::messages::start_transaction::{
    StartTransactionRequest, StartTransactionResponse,
};
use rust_ocpp::v1_6::types::{AuthorizationStatus, IdTagInfo};
use serde_json::Value;
use tracing::{error, info};

use crate::application::events::{Event, TransactionStartedEvent};
use crate::application::OcppHandlerV16;

pub async fn handle_start_transaction(handler: &OcppHandlerV16, payload: &Value) -> Value {
    let req: StartTransactionRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(charge_point_id = handler.charge_point_id.as_str(), error = %e, "Failed to parse StartTransaction");
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        connector_id = req.connector_id,
        id_tag = req.id_tag.as_str(),
        meter_start = req.meter_start,
        "StartTransaction"
    );

    let is_valid = handler
        .service
        .authorize(&req.id_tag)
        .await
        .unwrap_or(false);

    if !is_valid {
        let response = StartTransactionResponse {
            transaction_id: 0,
            id_tag_info: IdTagInfo {
                status: AuthorizationStatus::Invalid,
                expiry_date: None,
                parent_id_tag: None,
            },
        };
        return serde_json::to_value(&response).unwrap_or_default();
    }

    match handler
        .service
        .start_transaction(
            &handler.charge_point_id,
            req.connector_id,
            &req.id_tag,
            req.meter_start,
        )
        .await
    {
        Ok(transaction) => {
            handler.event_bus.publish(Event::TransactionStarted(TransactionStartedEvent {
                charge_point_id: handler.charge_point_id.clone(),
                connector_id: req.connector_id,
                transaction_id: transaction.id,
                id_tag: req.id_tag.clone(),
                meter_start: req.meter_start,
                timestamp: req.timestamp,
            }));

            let response = StartTransactionResponse {
                transaction_id: transaction.id,
                id_tag_info: IdTagInfo {
                    status: AuthorizationStatus::Accepted,
                    expiry_date: None,
                    parent_id_tag: None,
                },
            };
            serde_json::to_value(&response).unwrap_or_default()
        }
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "Failed to start transaction"
            );
            let response = StartTransactionResponse {
                transaction_id: 0,
                id_tag_info: IdTagInfo {
                    status: AuthorizationStatus::Invalid,
                    expiry_date: None,
                    parent_id_tag: None,
                },
            };
            serde_json::to_value(&response).unwrap_or_default()
        }
    }
}
