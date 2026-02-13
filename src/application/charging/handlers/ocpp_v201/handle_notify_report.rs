//! V201 NotifyReport handler
//!
//! Receives multi-part device reports from charge points in response
//! to a GetBaseReport command. Aggregates parts in the DeviceReportStore.

use rust_ocpp::v2_0_1::messages::notify_report::{NotifyReportRequest, NotifyReportResponse};
use serde_json::Value;
use tracing::{error, info};

use crate::application::charging::services::device_report::{
    ReportVariable, VariableAttributeEntry,
};
use crate::application::OcppHandlerV201;

pub async fn handle_notify_report(
    handler: &OcppHandlerV201,
    payload: &Value,
) -> Value {
    let req: NotifyReportRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse NotifyReport"
            );
            return serde_json::json!({});
        }
    };

    let tbc = req.tbc.unwrap_or(false);
    let var_count = req
        .report_data
        .as_ref()
        .map(|v| v.len())
        .unwrap_or(0);

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        request_id = req.request_id,
        seq_no = req.seq_no,
        tbc,
        variables = var_count,
        "V201 NotifyReport received (part {}{})",
        req.seq_no,
        if tbc { ", more coming" } else { ", final" }
    );

    // Convert ReportDataType → ReportVariable
    let variables: Vec<ReportVariable> = req
        .report_data
        .unwrap_or_default()
        .into_iter()
        .map(|rd| {
            let evse_id = rd.component.evse.as_ref().map(|e| e.id);
            let connector_id = rd.component.evse.as_ref().and_then(|e| e.connector_id);

            let attributes = rd
                .variable_attribute
                .into_iter()
                .map(|va| VariableAttributeEntry {
                    attr_type: va
                        .kind
                        .map(|k| format!("{:?}", k))
                        .unwrap_or_else(|| "Actual".to_string()),
                    value: va.value,
                    mutability: va.mutability.map(|m| format!("{:?}", m)),
                })
                .collect();

            let (data_type, unit) = rd
                .variable_characteristics
                .map(|vc| (Some(format!("{:?}", vc.data_type)), vc.unit))
                .unwrap_or((None, None));

            ReportVariable {
                component: rd.component.name,
                component_instance: rd.component.instance,
                evse_id,
                connector_id,
                variable: rd.variable.name,
                variable_instance: rd.variable.instance,
                attributes,
                data_type,
                unit,
            }
        })
        .collect();

    handler.report_store.append_report(
        &handler.charge_point_id,
        req.request_id,
        variables,
        tbc,
    );

    if !tbc {
        if let Some(report) = handler.report_store.get_report(&handler.charge_point_id, req.request_id) {
            info!(
                charge_point_id = handler.charge_point_id.as_str(),
                request_id = req.request_id,
                total_parts = report.parts_received,
                total_variables = report.variables.len(),
                "V201 NotifyReport complete — full report assembled"
            );
        }
    }

    serde_json::to_value(&NotifyReportResponse {}).unwrap_or_default()
}
