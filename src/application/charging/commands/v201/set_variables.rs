//! v2.0.1 SetVariables command
//!
//! Replaces v1.6's ChangeConfiguration — completely different structure.
//! Instead of key-value pairs, v2.0.1 uses component + variable addressing.

use rust_ocpp::v2_0_1::messages::set_variables::{SetVariablesRequest, SetVariablesResponse};
use rust_ocpp::v2_0_1::datatypes::component_type::ComponentType;
use rust_ocpp::v2_0_1::datatypes::set_variable_data_type::SetVariableDataType;
use rust_ocpp::v2_0_1::datatypes::variable_type::VariableType;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// A single variable set result
#[derive(Debug, Clone)]
pub struct SetVariableStatus {
    pub component: String,
    pub variable: String,
    pub status: String,
}

/// Result of a SetVariables command
#[derive(Debug)]
pub struct SetVariablesResult {
    pub results: Vec<SetVariableStatus>,
}

/// Set variables on a v2.0.1 charging station.
///
/// `variables` is a list of (component_name, variable_name, value) tuples.
pub async fn set_variables(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    variables: Vec<(String, String, String)>,
) -> Result<SetVariablesResult, CommandError> {
    info!(charge_point_id, count = variables.len(), "v2.0.1 SetVariables");

    let set_variable_data: Vec<SetVariableDataType> = variables
        .into_iter()
        .map(|(component, variable, value)| SetVariableDataType {
            attribute_type: None,
            attribute_value: value,
            component: ComponentType {
                name: component,
                instance: None,
                evse: None,
            },
            variable: VariableType {
                name: variable,
                instance: None,
            },
        })
        .collect();

    let request = SetVariablesRequest { set_variable_data };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "SetVariables", payload)
        .await?;

    let response: SetVariablesResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    let results = response
        .set_variable_result
        .into_iter()
        .map(|r| SetVariableStatus {
            component: r.component.name,
            variable: r.variable.name,
            status: format!("{:?}", r.attribute_status),
        })
        .collect();

    Ok(SetVariablesResult { results })
}
