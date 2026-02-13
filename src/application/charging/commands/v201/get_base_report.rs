//! v2.0.1 GetBaseReport command

use rust_ocpp::v2_0_1::enumerations::report_base_enum_type::ReportBaseEnumType;
use rust_ocpp::v2_0_1::messages::get_base_report::GetBaseReportRequest;
use rust_ocpp::v2_0_1::messages::get_base_report::GetBaseReportResponse;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Result of a GetBaseReport command.
#[derive(Debug, Clone)]
pub struct GetBaseReportResult {
    pub status: String,
}

pub async fn get_base_report(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    request_id: i32,
    report_base: &str,
) -> Result<GetBaseReportResult, CommandError> {
    info!(
        charge_point_id,
        request_id, report_base, "v2.0.1 GetBaseReport"
    );

    let report_base_enum = match report_base {
        "ConfigurationInventory" => ReportBaseEnumType::ConfigurationInventory,
        "SummaryInventory" => ReportBaseEnumType::SummaryInventory,
        _ => ReportBaseEnumType::FullInventory,
    };

    let request = GetBaseReportRequest {
        request_id,
        report_base: report_base_enum,
    };

    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "GetBaseReport", payload)
        .await?;

    let response: GetBaseReportResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(GetBaseReportResult {
        status: format!("{:?}", response.status),
    })
}
