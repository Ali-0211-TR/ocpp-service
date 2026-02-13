//! v2.0.1 ClearVariableMonitoring command

use rust_ocpp::v2_0_1::messages::clear_variable_monitoring::{
    ClearVariableMonitoringRequest, ClearVariableMonitoringResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// A single clear-monitor result entry.
#[derive(Debug, Clone)]
pub struct ClearMonitoringEntry {
    pub id: i32,
    pub status: String,
}

/// Result of a ClearVariableMonitoring command.
#[derive(Debug, Clone)]
pub struct ClearVariableMonitoringResult {
    pub results: Vec<ClearMonitoringEntry>,
}

pub async fn clear_variable_monitoring(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    ids: Vec<i32>,
) -> Result<ClearVariableMonitoringResult, CommandError> {
    info!(
        charge_point_id,
        monitor_ids = ?ids,
        "v2.0.1 ClearVariableMonitoring"
    );

    let request = ClearVariableMonitoringRequest { id: ids };

    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "ClearVariableMonitoring", payload)
        .await?;

    let response: ClearVariableMonitoringResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    let results = response
        .clear_monitoring_result
        .into_iter()
        .map(|r| ClearMonitoringEntry {
            id: r.id,
            status: format!("{:?}", r.status),
        })
        .collect();

    Ok(ClearVariableMonitoringResult { results })
}
