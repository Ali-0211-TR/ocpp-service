//! Identity module — user management & authentication
//!
//! Contains the `UserService` which orchestrates all user-related
//! use-cases: login, registration, profile updates, password changes.

pub mod queries;
pub mod service;

pub use service::{role_to_str, str_to_role, AuthResult, UserService};
