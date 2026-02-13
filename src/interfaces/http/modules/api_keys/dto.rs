//! API Key DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({
    "name": "My Integration",
    "scopes": ["charge_points:read", "transactions:read"]
}))]
pub struct CreateApiKeyRequest {
    #[validate(length(min = 1, max = 100, message = "name is required"))]
    pub name: String,
    #[validate(length(min = 1, message = "at least one scope is required"))]
    pub scopes: Vec<String>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub is_active: bool,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedApiKeyResponse {
    pub key: String,
    pub api_key: ApiKeyResponse,
}
