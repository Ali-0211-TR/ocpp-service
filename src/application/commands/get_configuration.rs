//! Get Configuration command

use log::info;
use ocpp_rs::v16::call::{Action, GetConfiguration};
use ocpp_rs::v16::call_result::ResultPayload;
use ocpp_rs::v16::data_types::KeyValue;

use super::{CommandError, SharedCommandSender};

/// Configuration result from charge point
#[derive(Debug)]
pub struct ConfigurationResult {
    /// Known configuration keys with their values
    pub configuration_key: Vec<KeyValue>,
    /// Unknown configuration keys
    pub unknown_key: Vec<String>,
}

/// Get configuration from a charge point
/// 
/// If keys is None or empty, returns all configuration
pub async fn get_configuration(
    command_sender: &SharedCommandSender,
    charge_point_id: &str,
    keys: Option<Vec<String>>,
) -> Result<ConfigurationResult, CommandError> {
    info!(
        "[{}] GetConfiguration - Keys: {:?}",
        charge_point_id, keys
    );

    let action = Action::GetConfiguration(GetConfiguration { key: keys });

    let result = command_sender.send_command(charge_point_id, action).await?;

    match result {
        ResultPayload::PossibleEmptyResponse(empty_response) => {
            match empty_response {
                ocpp_rs::v16::call_result::EmptyResponses::GetConfiguration(resp) => {
                    Ok(ConfigurationResult {
                        configuration_key: resp.configuration_key.unwrap_or_default(),
                        unknown_key: resp.unknown_key.unwrap_or_default(),
                    })
                }
                _ => Err(CommandError::InvalidResponse(
                    "Unexpected response type".to_string(),
                )),
            }
        }
        _ => Err(CommandError::InvalidResponse(
            "Unexpected response type".to_string(),
        )),
    }
}
