//! Infrastructure layer - external concerns

pub mod database;
pub mod server;
pub mod storage;

pub use database::{init_database, DatabaseConfig, DatabaseStorage};
pub use server::{OcppServer, ShutdownCoordinator, ShutdownSignal};
pub use storage::{InMemoryStorage, Storage};
