//! OCPP v1.6 command implementations
//!
//! Each function constructs a v1.6-specific request, sends it via
//! [`CommandSender`](super::CommandSender), and deserialises the v1.6 response.

pub mod change_availability;
pub mod cancel_reservation;
pub mod change_configuration;
pub mod clear_cache;
pub mod clear_charging_profile;
pub mod data_transfer;
pub mod get_composite_schedule;
pub mod get_configuration;
pub mod get_local_list_version;
pub mod remote_start;
pub mod remote_stop;
pub mod reserve_now;
pub mod reset;
pub mod send_local_list;
pub mod set_charging_profile;
pub mod trigger_message;
pub mod unlock_connector;
