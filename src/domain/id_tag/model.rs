//! IdTag domain entity

use chrono::{DateTime, Utc};

/// IdTag authorization status (OCPP 1.6)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdTagStatus {
    Accepted,
    Blocked,
    Expired,
    Invalid,
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

impl From<&str> for IdTagStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "accepted" => Self::Accepted,
            "blocked" => Self::Blocked,
            "expired" => Self::Expired,
            "invalid" => Self::Invalid,
            "concurrenttx" => Self::ConcurrentTx,
            _ => Self::Invalid,
        }
    }
}

/// RFID card / authorization token
#[derive(Debug, Clone)]
pub struct IdTag {
    /// The ID tag value (RFID card number)
    pub id_tag: String,
    /// Parent ID tag (for group authorization)
    pub parent_id_tag: Option<String>,
    /// Current status
    pub status: IdTagStatus,
    /// Optional user ID this tag belongs to
    pub user_id: Option<String>,
    /// Display name
    pub name: Option<String>,
    /// Expiry date
    pub expiry_date: Option<DateTime<Utc>>,
    /// Maximum active transactions allowed
    pub max_active_transactions: Option<i32>,
    /// Whether the tag is active
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl IdTag {
    pub fn new(id_tag: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id_tag: id_tag.into(),
            parent_id_tag: None,
            status: IdTagStatus::Accepted,
            user_id: None,
            name: None,
            expiry_date: None,
            max_active_transactions: None,
            is_active: true,
            created_at: now,
            updated_at: now,
            last_used_at: None,
        }
    }

    /// Check if this tag is currently valid for authorization
    pub fn is_valid(&self) -> bool {
        if !self.is_active {
            return false;
        }
        if self.status != IdTagStatus::Accepted {
            return false;
        }
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
