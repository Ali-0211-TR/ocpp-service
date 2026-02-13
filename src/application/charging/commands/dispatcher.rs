//! Command dispatcher — resolves OCPP version and delegates to v16 or v201 implementations.
//!
//! The dispatcher is the single entry point for HTTP handlers to send CS→CP commands.
//! It looks up the charge-point's negotiated OCPP version via
//! [`SessionRegistry`] and dispatches to the correct version-specific module.

use std::sync::Arc;

use tracing::info;

use super::v201;
use super::{v16, CommandError, SharedCommandSender};
use crate::application::charging::session::SharedSessionRegistry;
use crate::domain::OcppVersion;

// Re-export common types used by the dispatcher's public API
pub use super::{
    Availability, ConfigurationResult, DataTransferResult, KeyValue, ResetKind, TriggerType,
};
pub use v201::get_variables::GetVariablesResult;
pub use v201::set_variables::SetVariablesResult;

/// Record command dispatch latency to Prometheus.
fn record_command_latency(action: &'static str, start: std::time::Instant) {
    let duration = start.elapsed().as_secs_f64();
    metrics::histogram!("ocpp_command_latency_seconds", "action" => action).record(duration);
    metrics::counter!("ocpp_commands_total", "action" => action).increment(1);
}

/// Version-aware command dispatcher.
///
/// Wraps [`CommandSender`] (the low-level transport layer) and
/// [`SessionRegistry`] (to resolve the active OCPP version).
pub struct CommandDispatcher {
    command_sender: SharedCommandSender,
    session_registry: SharedSessionRegistry,
}

impl CommandDispatcher {
    pub fn new(
        command_sender: SharedCommandSender,
        session_registry: SharedSessionRegistry,
    ) -> Self {
        Self {
            command_sender,
            session_registry,
        }
    }

    /// Resolve the OCPP version for a connected charge point.
    fn resolve_version(&self, charge_point_id: &str) -> Result<OcppVersion, CommandError> {
        self.session_registry
            .get_version(charge_point_id)
            .ok_or_else(|| {
                CommandError::NotConnected(format!(
                    "Charge point '{}' is not connected or version unknown",
                    charge_point_id
                ))
            })
    }

    /// Get a reference to the underlying command sender (for low-level use).
    pub fn command_sender(&self) -> &SharedCommandSender {
        &self.command_sender
    }

    // ─── Remote Start Transaction ──────────────────────────────────────

    pub async fn remote_start(
        &self,
        charge_point_id: &str,
        id_tag: &str,
        connector_id: Option<u32>,
    ) -> Result<String, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();
        info!(%version, "Dispatching RemoteStart");

        let result = match version {
            OcppVersion::V16 => {
                v16::remote_start::remote_start_transaction(
                    &self.command_sender,
                    charge_point_id,
                    id_tag,
                    connector_id,
                )
                .await
            }
            OcppVersion::V201 | OcppVersion::V21 => {
                let evse_id = connector_id.map(|c| c as i32);
                v201::remote_start::remote_start_transaction(
                    &self.command_sender,
                    charge_point_id,
                    id_tag,
                    evse_id,
                )
                .await
            }
        };
        record_command_latency("remote_start", start);
        result
    }

    // ─── Remote Stop Transaction ───────────────────────────────────────

    pub async fn remote_stop(
        &self,
        charge_point_id: &str,
        transaction_id: i32,
    ) -> Result<String, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();
        info!(%version, "Dispatching RemoteStop");

        let result = match version {
            OcppVersion::V16 => {
                v16::remote_stop::remote_stop_transaction(
                    &self.command_sender,
                    charge_point_id,
                    transaction_id,
                )
                .await
            }
            OcppVersion::V201 | OcppVersion::V21 => {
                let tx_id = transaction_id.to_string();
                v201::remote_stop::remote_stop_transaction(
                    &self.command_sender,
                    charge_point_id,
                    &tx_id,
                )
                .await
            }
        };
        record_command_latency("remote_stop", start);
        result
    }

    // ─── Reset ─────────────────────────────────────────────────────────

    pub async fn reset(
        &self,
        charge_point_id: &str,
        reset_type: ResetKind,
    ) -> Result<String, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();
        info!(%version, "Dispatching Reset");

        let result = match version {
            OcppVersion::V16 => {
                v16::reset::reset(&self.command_sender, charge_point_id, reset_type).await
            }
            OcppVersion::V201 | OcppVersion::V21 => {
                v201::reset::reset(&self.command_sender, charge_point_id, reset_type, None).await
            }
        };
        record_command_latency("reset", start);
        result
    }

    // ─── Change Availability ───────────────────────────────────────────

    pub async fn change_availability(
        &self,
        charge_point_id: &str,
        connector_id: u32,
        availability: Availability,
    ) -> Result<String, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();
        info!(%version, "Dispatching ChangeAvailability");

        let result = match version {
            OcppVersion::V16 => {
                v16::change_availability::change_availability(
                    &self.command_sender,
                    charge_point_id,
                    connector_id,
                    availability,
                )
                .await
            }
            OcppVersion::V201 | OcppVersion::V21 => {
                // In v2.0.1: connector_id maps to evse_id
                v201::change_availability::change_availability(
                    &self.command_sender,
                    charge_point_id,
                    connector_id as i32,
                    None,
                    availability,
                )
                .await
            }
        };
        record_command_latency("change_availability", start);
        result
    }

    // ─── Change Configuration (v1.6 only) ──────────────────────────────

    /// ChangeConfiguration — v1.6 only.
    /// For v2.0.1, use [`set_variables`] instead.
    pub async fn change_configuration(
        &self,
        charge_point_id: &str,
        key: String,
        value: String,
    ) -> Result<String, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();

        let result = match version {
            OcppVersion::V16 => {
                v16::change_configuration::change_configuration(
                    &self.command_sender,
                    charge_point_id,
                    key,
                    value,
                )
                .await
            }
            OcppVersion::V201 | OcppVersion::V21 => Err(CommandError::UnsupportedVersion(
                "ChangeConfiguration is not available in OCPP 2.0.1. Use SetVariables instead."
                    .to_string(),
            )),
        };
        record_command_latency("change_configuration", start);
        result
    }

    // ─── Get Configuration (v1.6 only) ─────────────────────────────────

    /// GetConfiguration — v1.6 only.
    /// For v2.0.1, use [`get_variables`] instead.
    pub async fn get_configuration(
        &self,
        charge_point_id: &str,
        keys: Option<Vec<String>>,
    ) -> Result<ConfigurationResult, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();

        let result = match version {
            OcppVersion::V16 => {
                v16::get_configuration::get_configuration(
                    &self.command_sender,
                    charge_point_id,
                    keys,
                )
                .await
            }
            OcppVersion::V201 | OcppVersion::V21 => Err(CommandError::UnsupportedVersion(
                "GetConfiguration is not available in OCPP 2.0.1. Use GetVariables instead."
                    .to_string(),
            )),
        };
        record_command_latency("get_configuration", start);
        result
    }

    // ─── Get Variables (v2.0.1 only) ───────────────────────────────────

    /// GetVariables — v2.0.1 only (replaces GetConfiguration).
    pub async fn get_variables(
        &self,
        charge_point_id: &str,
        variables: Vec<(String, String)>,
    ) -> Result<GetVariablesResult, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();

        let result = match version {
            OcppVersion::V16 => Err(CommandError::UnsupportedVersion(
                "GetVariables is not available in OCPP 1.6. Use GetConfiguration instead."
                    .to_string(),
            )),
            OcppVersion::V201 | OcppVersion::V21 => {
                v201::get_variables::get_variables(
                    &self.command_sender,
                    charge_point_id,
                    variables,
                )
                .await
            }
        };
        record_command_latency("get_variables", start);
        result
    }

    // ─── Set Variables (v2.0.1 only) ───────────────────────────────────

    /// SetVariables — v2.0.1 only (replaces ChangeConfiguration).
    pub async fn set_variables(
        &self,
        charge_point_id: &str,
        variables: Vec<(String, String, String)>,
    ) -> Result<SetVariablesResult, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();

        let result = match version {
            OcppVersion::V16 => Err(CommandError::UnsupportedVersion(
                "SetVariables is not available in OCPP 1.6. Use ChangeConfiguration instead."
                    .to_string(),
            )),
            OcppVersion::V201 | OcppVersion::V21 => {
                v201::set_variables::set_variables(
                    &self.command_sender,
                    charge_point_id,
                    variables,
                )
                .await
            }
        };
        record_command_latency("set_variables", start);
        result
    }

    // ─── Clear Cache ───────────────────────────────────────────────────

    pub async fn clear_cache(
        &self,
        charge_point_id: &str,
    ) -> Result<String, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();
        info!(%version, "Dispatching ClearCache");

        let result = match version {
            OcppVersion::V16 => {
                v16::clear_cache::clear_cache(&self.command_sender, charge_point_id).await
            }
            OcppVersion::V201 | OcppVersion::V21 => {
                v201::clear_cache::clear_cache(&self.command_sender, charge_point_id).await
            }
        };
        record_command_latency("clear_cache", start);
        result
    }

    // ─── Data Transfer ─────────────────────────────────────────────────

    pub async fn data_transfer(
        &self,
        charge_point_id: &str,
        vendor_id: String,
        message_id: Option<String>,
        data: Option<String>,
    ) -> Result<DataTransferResult, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();
        info!(%version, "Dispatching DataTransfer");

        let result = match version {
            OcppVersion::V16 => {
                v16::data_transfer::data_transfer(
                    &self.command_sender,
                    charge_point_id,
                    vendor_id,
                    message_id,
                    data,
                )
                .await
            }
            OcppVersion::V201 | OcppVersion::V21 => {
                v201::data_transfer::data_transfer(
                    &self.command_sender,
                    charge_point_id,
                    vendor_id,
                    message_id,
                    data,
                )
                .await
            }
        };
        record_command_latency("data_transfer", start);
        result
    }

    // ─── Get Local List Version ────────────────────────────────────────

    pub async fn get_local_list_version(
        &self,
        charge_point_id: &str,
    ) -> Result<i32, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();
        info!(%version, "Dispatching GetLocalListVersion");

        let result = match version {
            OcppVersion::V16 => {
                v16::get_local_list_version::get_local_list_version(
                    &self.command_sender,
                    charge_point_id,
                )
                .await
            }
            OcppVersion::V201 | OcppVersion::V21 => {
                v201::get_local_list_version::get_local_list_version(
                    &self.command_sender,
                    charge_point_id,
                )
                .await
            }
        };
        record_command_latency("get_local_list_version", start);
        result
    }

    // ─── Trigger Message ───────────────────────────────────────────────

    pub async fn trigger_message(
        &self,
        charge_point_id: &str,
        requested_message: TriggerType,
        connector_id: Option<u32>,
    ) -> Result<String, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();
        info!(%version, "Dispatching TriggerMessage");

        let result = match version {
            OcppVersion::V16 => {
                v16::trigger_message::trigger_message(
                    &self.command_sender,
                    charge_point_id,
                    requested_message,
                    connector_id,
                )
                .await
            }
            OcppVersion::V201 | OcppVersion::V21 => {
                let evse_id = connector_id.map(|c| c as i32);
                v201::trigger_message::trigger_message(
                    &self.command_sender,
                    charge_point_id,
                    requested_message,
                    evse_id,
                )
                .await
            }
        };
        record_command_latency("trigger_message", start);
        result
    }

    // ─── Unlock Connector ──────────────────────────────────────────────

    pub async fn unlock_connector(
        &self,
        charge_point_id: &str,
        connector_id: u32,
    ) -> Result<String, CommandError> {
        let version = self.resolve_version(charge_point_id)?;
        let start = std::time::Instant::now();
        info!(%version, "Dispatching UnlockConnector");

        let result = match version {
            OcppVersion::V16 => {
                v16::unlock_connector::unlock_connector(
                    &self.command_sender,
                    charge_point_id,
                    connector_id,
                )
                .await
            }
            OcppVersion::V201 | OcppVersion::V21 => {
                // In v2.0.1: connector_id 1 maps to evse_id=1, connector_id=1
                v201::unlock_connector::unlock_connector(
                    &self.command_sender,
                    charge_point_id,
                    connector_id as i32, // evse_id
                    1,                   // connector_id within EVSE
                )
                .await
            }
        };
        record_command_latency("unlock_connector", start);
        result
    }
}

pub type SharedCommandDispatcher = Arc<CommandDispatcher>;

pub fn create_command_dispatcher(
    command_sender: SharedCommandSender,
    session_registry: SharedSessionRegistry,
) -> SharedCommandDispatcher {
    Arc::new(CommandDispatcher::new(command_sender, session_registry))
}
