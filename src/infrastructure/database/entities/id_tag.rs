//! IdTag entity for database

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// IdTag status
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum IdTagStatus {
    #[sea_orm(string_value = "Accepted")]
    Accepted,
    #[sea_orm(string_value = "Blocked")]
    Blocked,
    #[sea_orm(string_value = "Expired")]
    Expired,
    #[sea_orm(string_value = "Invalid")]
    Invalid,
    #[sea_orm(string_value = "ConcurrentTx")]
    ConcurrentTx,
}

impl Default for IdTagStatus {
    fn default() -> Self {
        Self::Accepted
    }
}

impl std::fmt::Display for IdTagStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accepted => write!(f, "Accepted"),
            Self::Blocked => write!(f, "Blocked"),
            Self::Expired => write!(f, "Expired"),
            Self::Invalid => write!(f, "Invalid"),
            Self::ConcurrentTx => write!(f, "ConcurrentTx"),
        }
    }
}

/// IdTag model - represents RFID cards/tokens for authorization
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "id_tags")]
pub struct Model {
    /// The ID tag value (RFID card number)
    #[sea_orm(primary_key, auto_increment = false)]
    pub id_tag: String,
    
    /// Parent ID tag (for group authorization)
    pub parent_id_tag: Option<String>,
    
    /// Current status of the tag
    pub status: IdTagStatus,
    
    /// Optional user ID this tag belongs to
    pub user_id: Option<String>,
    
    /// Display name for the tag
    pub name: Option<String>,
    
    /// Expiry date of the tag
    pub expiry_date: Option<DateTime<Utc>>,
    
    /// Maximum active transactions allowed (None = unlimited)
    pub max_active_transactions: Option<i32>,
    
    /// Whether the tag is active
    pub is_active: bool,
    
    /// When the tag was created
    pub created_at: DateTime<Utc>,
    
    /// When the tag was last updated
    pub updated_at: DateTime<Utc>,
    
    /// Last time this tag was used for authorization
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_delete = "SetNull"
    )]
    User,
    
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentIdTag",
        to = "Column::IdTag",
        on_delete = "SetNull"
    )]
    ParentTag,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Check if this tag is currently valid for authorization
    pub fn is_valid(&self) -> bool {
        if !self.is_active {
            return false;
        }
        
        if self.status != IdTagStatus::Accepted {
            return false;
        }
        
        // Check expiry
        if let Some(expiry) = self.expiry_date {
            if Utc::now() > expiry {
                return false;
            }
        }
        
        true
    }
    
    /// Get the OCPP authorization status for this tag
    pub fn get_auth_status(&self) -> IdTagStatus {
        if !self.is_active {
            return IdTagStatus::Invalid;
        }
        
        if let Some(expiry) = self.expiry_date {
            if Utc::now() > expiry {
                return IdTagStatus::Expired;
            }
        }
        
        self.status.clone()
    }
}
