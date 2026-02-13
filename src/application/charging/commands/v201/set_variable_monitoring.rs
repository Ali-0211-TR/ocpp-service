//! v2.0.1 SetVariableMonitoring command

use rust_ocpp::v2_0_1::datatypes::component_type::ComponentType;
use rust_ocpp::v2_0_1::datatypes::set_monitoring_data_type::SetMonitoringDataType;
use rust_ocpp::v2_0_1::datatypes::variable_type::VariableType;
use rust_ocpp::v2_0_1::enumerations::monitor_enum_type::MonitorEnumType;
use rust_ocpp::v2_0_1::messages::set_variable_monitoring::{
    SetVariableMonitoringRequest, SetVariableMonitoringResponse,
};
use rust_decimal::Decimal;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// A single monitoring-set result entry.
#[derive(Debug, Clone)]
pub struct MonitoringResultEntry {
    pub component: String,
    pub variable: String,
    pub status: String,
    pub monitor_id: Option<i32>,
    pub monitor_type: String,
}

/// Result of a SetVariableMonitoring command.
#[derive(Debug, Clone)]
pub struct SetVariableMonitoringResult {
    pub results: Vec<MonitoringResultEntry>,
}

/// Descriptor for a single monitor to set.
#[derive(Debug, Clone)]
pub struct MonitorDescriptor {
    pub component: String,
    pub variable: String,
    pub monitor_type: String,
    pub value: f64,
    pub severity: u8,
    pub transaction: Option<bool>,
    pub id: Option<i32>,
}

fn parse_monitor_type(s: &str) -> MonitorEnumType {
    match s {
        "UpperThreshold" => MonitorEnumType::UpperThreshold,
        "LowerThreshold" => MonitorEnumType::LowerThreshold,
        "Delta" => MonitorEnumType::Delta,
        "Periodic" => MonitorEnumType::Periodic,
        "PeriodicClockAligned" => MonitorEnumType::PeriodicClockAligned,
        _ => MonitorEnumType::UpperThreshold,
    }
}

pub async fn set_variable_monitoring(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    monitors: Vec<MonitorDescriptor>,
) -> Result<SetVariableMonitoringResult, CommandError> {
    info!(
        charge_point_id,
        monitors = monitors.len(),
        "v2.0.1 SetVariableMonitoring"
    );

    let set_monitoring_data: Vec<SetMonitoringDataType> = monitors
        .iter()
        .map(|m| SetMonitoringDataType {
            id: m.id,
            transaction: m.transaction,
            value: Decimal::from_f64_retain(m.value).unwrap_or_default(),
            kind: parse_monitor_type(&m.monitor_type),
            severity: m.severity,
            component: ComponentType {
                name: m.component.clone(),
                instance: None,
                evse: None,
            },
            variable: VariableType {
                name: m.variable.clone(),
                instance: None,
            },
        })
        .collect();

    let request = SetVariableMonitoringRequest {
        set_monitoring_data,
    };

    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "SetVariableMonitoring", payload)
        .await?;

    let response: SetVariableMonitoringResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    let results = response
        .set_monitoring_result
        .into_iter()
        .map(|r| MonitoringResultEntry {
            component: r.component.name,
            variable: r.variable.name,
            status: format!("{:?}", r.status),
            monitor_id: r.id,
            monitor_type: format!("{:?}", r.kind),
        })
        .collect();

    Ok(SetVariableMonitoringResult { results })
}
