pub mod database;
pub mod crypto;

// Re-export commonly used types
pub use database::storage::DatabaseStorage;
pub use database::{init_database, DatabaseConfig};
pub use crate::domain::Storage;