//! IdTag aggregate
//!
//! Contains the IdTag entity, authorization logic, and repository interface.

pub mod model;
pub mod repository;

pub use model::{IdTag, IdTagStatus};
pub use repository::IdTagRepository;
