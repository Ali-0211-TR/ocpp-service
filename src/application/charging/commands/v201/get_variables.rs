//! v2.0.1 GetVariables command
//!
//! Replaces v1.6's GetConfiguration — completely different structure.
//! Instead of key-value pairs, v2.0.1 uses component + variable addressing.

use rust_ocpp::v2_0_1::messages::get_variables::{GetVariablesRequest, GetVariablesResponse};
use rust_ocpp::v2_0_1::datatypes::component_type::ComponentType;
use rust_ocpp::v2_0_1::datatypes::get_variable_data_type::GetVariableDataType;
use rust_ocpp::v2_0_1::datatypes::variable_type::VariableType;
use tracing::info;

use crate::application::charging::commands::{CommandError, SharedCommandSender};

/// A single variable result from GetVariables
#[derive(Debug, Clone)]
pub struct VariableResult {
    pub component: String,
    pub variable: String,
    pub attribute_status: String,
    pub attribute_value: Option<String>,
}

/// Result of a GetVariables command
#[derive(Debug)]
pub struct GetVariablesResult {
    pub results: Vec<VariableResult>,
}

/// Get variables from a v2.0.1 charging station.
///
/// `variables` is a list of (component_name, variable_name) pairs.
pub async fn get_variables(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    variables: Vec<(String, String)>,
) -> Result<GetVariablesResult, CommandError> {
    info!(charge_point_id, count = variables.len(), "v2.0.1 GetVariables");

    let get_variable_data: Vec<GetVariableDataType> = variables
        .into_iter()
        .map(|(component, variable)| GetVariableDataType {
            attribute_type: None,
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

    let request = GetVariablesRequest { get_variable_data };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "GetVariables", payload)
        .await?;

    let response: GetVariablesResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    let results = response
        .get_variable_result
        .into_iter()
        .map(|r| VariableResult {
            component: r.component.name,
            variable: r.variable.name,
            attribute_status: format!("{:?}", r.attribute_status),
            attribute_value: r.attribute_value,
        })
        .collect();

    Ok(GetVariablesResult { results })
}
