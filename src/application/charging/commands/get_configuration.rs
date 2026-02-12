//! Get Configuration command

use rust_ocpp::v1_6::messages::get_configuration::{
    GetConfigurationRequest, GetConfigurationResponse,
};
use tracing::info;

use super::{CommandError, SharedCommandSender};

/// A configuration key-value pair returned by GetConfiguration
#[derive(Debug, Clone)]
pub struct KeyValue {
    pub key: String,
    pub readonly: bool,
    pub value: Option<String>,
}

#[derive(Debug)]
pub struct ConfigurationResult {
    pub configuration_key: Vec<KeyValue>,
    pub unknown_key: Vec<String>,
}

pub async fn get_configuration(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    keys: Option<Vec<String>>,
) -> Result<ConfigurationResult, CommandError> {
    info!(charge_point_id, ?keys, "GetConfiguration");

    let request = GetConfigurationRequest { key: keys };
    let payload = serde_json::to_value(&request)
        .map_err(|e| CommandError::SendFailed(format!("Serialization failed: {}", e)))?;

    let result = command_sender
        .send_command(charge_point_id, "GetConfiguration", payload)
        .await?;

    let response: GetConfigurationResponse = serde_json::from_value(result)
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    let configuration_key = response
        .configuration_key
        .unwrap_or_default()
        .into_iter()
        .map(|kv| KeyValue {
            key: kv.key,
            readonly: kv.readonly,
            value: kv.value,
        })
        .collect();

    Ok(ConfigurationResult {
        configuration_key,
        unknown_key: response.unknown_key.unwrap_or_default(),
    })
}
