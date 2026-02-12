pub mod crypto;
pub mod database;

// Re-export commonly used types
pub use crate::domain::Storage;
pub use database::storage::DatabaseStorage;
pub use database::{init_database, DatabaseConfig};
