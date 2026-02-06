//! Authentication and Authorization module
//!
//! Provides JWT token-based authentication and API key support.

pub mod jwt;
pub mod middleware;
pub mod password;
pub mod api_key;

pub use jwt::{Claims, JwtConfig, create_token, verify_token};
pub use middleware::{auth_middleware, optional_auth_middleware, api_key_middleware};
pub use password::{hash_password, verify_password};
pub use api_key::{generate_api_key, ApiKeyInfo};
