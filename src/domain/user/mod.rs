//! User aggregate
//!
//! Contains the User entity, DTOs, and repository interface.

pub mod model;
pub mod repository;

mod dto_change_password;
mod dto_create;
mod dto_get;
mod dto_update;

// Re-export model types
pub use model::{User, UserRole};

// Re-export DTOs
pub use dto_change_password::UserChangePasswordDto;
pub use dto_create::CreateUserDto;
pub use dto_get::GetUserDto;
pub use dto_update::UpdateUserDto;

// Re-export repository trait
pub use repository::UserRepositoryInterface;
