//! v2.0.1 SetMonitoringBase command

use rust_ocpp::v2_0_1::enumerations::monitoring_base_enum_type::MonitoringBaseEnumType;
use rust_ocpp::v2_0_1::messages::set_monitoring_base::{
    SetMonitoringBaseRequest, SetMonitoringBaseResponse,
};
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// Result of a SetMonitoringBase command.
#[derive(Debug, Clone)]
pub struct SetMonitoringBaseResult {
    pub status: String,
}

fn parse_monitoring_base(s: &str) -> MonitoringBaseEnumType {
    match s {
        "All" => MonitoringBaseEnumType::All,
        "FactoryDefault" => MonitoringBaseEnumType::FactoryDefault,
        "HardWiredOnly" => MonitoringBaseEnumType::HardWiredOnly,
        _ => MonitoringBaseEnumType::All,
    }
}

pub async fn set_monitoring_base(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    monitoring_base: &str,
) -> Result<SetMonitoringBaseResult, CommandError> {
    info!(
        charge_point_id,
        monitoring_base, "v2.0.1 SetMonitoringBase"
    );

    let request = SetMonitoringBaseRequest {
        monitoring_base: parse_monitoring_base(monitoring_base),
    };

    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "SetMonitoringBase", payload)
        .await?;

    let response: SetMonitoringBaseResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    Ok(SetMonitoringBaseResult {
        status: format!("{:?}", response.status),
    })
}
