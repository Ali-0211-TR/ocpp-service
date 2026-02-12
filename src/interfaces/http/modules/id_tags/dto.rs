//! IdTag DTOs

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::infrastructure::database::entities::id_tag;

#[derive(Debug, Serialize, ToSchema)]
pub struct IdTagDto {
    pub id_tag: String,
    pub parent_id_tag: Option<String>,
    pub status: String,
    pub user_id: Option<String>,
    pub name: Option<String>,
    pub expiry_date: Option<String>,
    pub max_active_transactions: Option<i32>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_used_at: Option<String>,
}

impl From<id_tag::Model> for IdTagDto {
    fn from(t: id_tag::Model) -> Self {
        Self {
            id_tag: t.id_tag,
            parent_id_tag: t.parent_id_tag,
            status: t.status.to_string(),
            user_id: t.user_id,
            name: t.name,
            expiry_date: t.expiry_date.map(|d| d.to_rfc3339()),
            max_active_transactions: t.max_active_transactions,
            is_active: t.is_active,
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
            last_used_at: t.last_used_at.map(|d| d.to_rfc3339()),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateIdTagRequest {
    pub id_tag: String,
    pub parent_id_tag: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
    pub user_id: Option<String>,
    pub name: Option<String>,
    pub expiry_date: Option<String>,
    pub max_active_transactions: Option<i32>,
}

fn default_status() -> String {
    "Accepted".to_string()
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateIdTagRequest {
    pub parent_id_tag: Option<String>,
    pub status: Option<String>,
    pub user_id: Option<String>,
    pub name: Option<String>,
    pub expiry_date: Option<String>,
    pub max_active_transactions: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListIdTagsParams {
    pub status: Option<String>,
    pub is_active: Option<bool>,
    pub user_id: Option<String>,
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_page() -> u64 {
    1
}
fn default_page_size() -> u64 {
    20
}
