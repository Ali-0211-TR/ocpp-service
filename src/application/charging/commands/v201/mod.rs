//! OCPP v2.0.1 command implementations
//!
//! Each function constructs a v2.0.1-specific request, sends it via
//! [`CommandSender`](super::CommandSender), and deserialises the v2.0.1 response.

pub mod cancel_reservation;
pub mod change_availability;
pub mod clear_cache;
pub mod clear_charging_profile;
pub mod data_transfer;
pub mod get_base_report;
pub mod get_composite_schedule;
pub mod get_local_list_version;
pub mod get_log;
pub mod get_variables;
pub mod remote_start;
pub mod remote_stop;
pub mod reserve_now;
pub mod reset;
pub mod send_local_list;
pub mod set_charging_profile;
pub mod set_variables;
pub mod trigger_message;
pub mod unlock_connector;
pub mod update_firmware;
