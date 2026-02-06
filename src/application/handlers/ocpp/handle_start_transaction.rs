//! StartTransaction handler

use log::{error, info};
use ocpp_rs::v16::call::StartTransaction;
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::data_types::IdTagInfo;
use ocpp_rs::v16::enums::ParsedGenericStatus;

use crate::application::OcppHandler;
use crate::notifications::{Event, TransactionStartedEvent};

pub async fn handle_start_transaction(
    handler: &OcppHandler,
    payload: StartTransaction,
) -> ResultPayload {
    info!(
        "[{}] StartTransaction - Connector: {}, IdTag: {}, MeterStart: {}",
        handler.charge_point_id, payload.connector_id, payload.id_tag, payload.meter_start
    );

    // Authorize the ID tag
    let is_valid = handler.service.authorize(&payload.id_tag).await.unwrap_or(false);

    if !is_valid {
        return ResultPayload::StartTransaction(ocpp_rs::v16::call_result::StartTransaction {
            transaction_id: 0,
            id_tag_info: IdTagInfo {
                status: ParsedGenericStatus::Invalid,
                expiry_date: None,
                parent_id_tag: None,
            },
        });
    }

    // Start transaction
    match handler
        .service
        .start_transaction(
            &handler.charge_point_id,
            payload.connector_id,
            &payload.id_tag,
            payload.meter_start as i32,
        )
        .await
    {
        Ok(transaction) => {
            // Publish transaction started event
            handler.event_bus.publish(Event::TransactionStarted(TransactionStartedEvent {
                charge_point_id: handler.charge_point_id.clone(),
                connector_id: payload.connector_id,
                transaction_id: transaction.id,
                id_tag: payload.id_tag.clone(),
                meter_start: payload.meter_start as i32,
                timestamp: payload.timestamp.inner(),
            }));

            ResultPayload::StartTransaction(ocpp_rs::v16::call_result::StartTransaction {
                transaction_id: transaction.id,
                id_tag_info: IdTagInfo {
                    status: ParsedGenericStatus::Accepted,
                    expiry_date: None,
                    parent_id_tag: None,
                },
            })
        }
        Err(e) => {
            error!("[{}] Failed to start transaction: {}", handler.charge_point_id, e);
            ResultPayload::StartTransaction(ocpp_rs::v16::call_result::StartTransaction {
                transaction_id: 0,
                id_tag_info: IdTagInfo {
                    status: ParsedGenericStatus::Invalid,
                    expiry_date: None,
                    parent_id_tag: None,
                },
            })
        }
    }
}