//! V201 DataTransfer handler
//!
//! In OCPP 2.0.1, the field is `vendor_id` (not `vendor_string` like V1.6).

use rust_ocpp::v2_0_1::enumerations::data_transfer_status_enum_type::DataTransferStatusEnumType;
use rust_ocpp::v2_0_1::messages::datatransfer::{DataTransferRequest, DataTransferResponse};
use serde_json::Value;
use tracing::{error, info};

use crate::application::handlers::OcppHandlerV201;

pub async fn handle_data_transfer(handler: &OcppHandlerV201, payload: &Value) -> Value {
    let req: DataTransferRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse DataTransfer"
            );
            return serde_json::json!({});
        }
    };

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        vendor_id = req.vendor_id.as_str(),
        message_id = ?req.message_id,
        "V201 DataTransfer"
    );

    let response = DataTransferResponse {
        status: DataTransferStatusEnumType::Accepted,
        data: None,
        status_info: None,
    };

    serde_json::to_value(&response).unwrap_or_default()
}
