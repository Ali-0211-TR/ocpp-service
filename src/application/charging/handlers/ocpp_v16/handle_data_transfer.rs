//! DataTransfer handler

use rust_ocpp::v1_6::messages::data_transfer::{DataTransferRequest, DataTransferResponse};
use rust_ocpp::v1_6::types::DataTransferStatus;
use serde_json::Value;
use tracing::{error, info};

use crate::application::OcppHandlerV16;

pub async fn handle_data_transfer(handler: &OcppHandlerV16, payload: &Value) -> Value {
    let req: DataTransferRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(charge_point_id = handler.charge_point_id.as_str(), error = %e, "Failed to parse DataTransfer");
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        vendor_id = req.vendor_string.as_str(),
        message_id = ?req.message_id,
        "DataTransfer"
    );

    let response = DataTransferResponse {
        status: DataTransferStatus::Accepted,
        data: None,
    };

    serde_json::to_value(&response).unwrap_or_default()
}
