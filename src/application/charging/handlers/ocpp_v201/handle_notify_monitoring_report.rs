//! V201 NotifyMonitoringReport handler
//!
//! Receives monitoring configuration reports from charge points in response
//! to GetMonitoringReport or similar CSMS-initiated requests.
//! Currently logs the data; a dedicated monitoring store could be added later.

use rust_ocpp::v2_0_1::messages::notify_monitoring_report::{
    NotifyMonitoringReportRequest, NotifyMonitoringReportResponse,
};
use serde_json::Value;
use tracing::{error, info};

use crate::application::OcppHandlerV201;

pub async fn handle_notify_monitoring_report(
    handler: &OcppHandlerV201,
    payload: &Value,
) -> Value {
    let req: NotifyMonitoringReportRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse NotifyMonitoringReport"
            );
            return serde_json::to_value(NotifyMonitoringReportResponse {}).unwrap_or_default();
        }
    };

    let tbc = req.tbc.unwrap_or(false);
    let monitor_count = req.monitor.as_ref().map(|v| v.len()).unwrap_or(0);

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        request_id = req.request_id,
        seq_no = req.seq_no,
        tbc,
        monitors = monitor_count,
        "V201 NotifyMonitoringReport received (part {}{})",
        req.seq_no,
        if tbc { ", more coming" } else { ", final" }
    );

    if let Some(monitors) = &req.monitor {
        for md in monitors {
            let component = &md.component.name;
            let variable = &md.variable.name;
            let monitor_count_inner = md.variable_monitoring.len();

            info!(
                charge_point_id = handler.charge_point_id.as_str(),
                component,
                variable,
                monitors = monitor_count_inner,
                "Monitoring data: {} / {} — {} monitor(s)",
                component,
                variable,
                monitor_count_inner
            );

            for vm in &md.variable_monitoring {
                info!(
                    charge_point_id = handler.charge_point_id.as_str(),
                    monitor_id = vm.id,
                    transaction = vm.transaction,
                    value = %vm.value,
                    monitor_type = ?vm.kind,
                    severity = vm.severity,
                    "  Monitor: id={}, type={:?}, value={}, severity={}",
                    vm.id,
                    vm.kind,
                    vm.value,
                    vm.severity
                );
            }
        }
    }

    serde_json::to_value(NotifyMonitoringReportResponse {}).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_monitoring_report_response_shape() {
        let resp = NotifyMonitoringReportResponse {};
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json, serde_json::json!({}));
    }
}
