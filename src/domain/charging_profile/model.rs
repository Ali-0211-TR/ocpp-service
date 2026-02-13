//! ChargingProfile domain entity

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Stored charging profile record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChargingProfile {
    /// Internal auto-increment ID (DB row).
    pub id: i32,
    /// Charge point this profile is installed on.
    pub charge_point_id: String,
    /// EVSE / connector ID (0 = station-wide).
    pub evse_id: i32,
    /// Profile ID from the OCPP ChargingProfile object.
    pub profile_id: i32,
    /// Stack level (higher = higher priority).
    pub stack_level: i32,
    /// Charging profile purpose (e.g. TxDefaultProfile, TxProfile, ChargingStationMaxProfile).
    pub purpose: String,
    /// Charging profile kind (Absolute, Recurring, Relative).
    pub kind: String,
    /// Recurrency kind (Daily, Weekly) — optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrency_kind: Option<String>,
    /// Valid-from timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<DateTime<Utc>>,
    /// Valid-to timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<DateTime<Utc>>,
    /// Full charging schedule(s) as JSON string.
    pub schedule_json: String,
    /// Whether this profile is currently active on the charge point.
    pub is_active: bool,
    /// When the profile was first sent.
    pub created_at: DateTime<Utc>,
    /// When the profile was last updated (e.g. deactivated).
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_charging_profile_serialization() {
        let profile = ChargingProfile {
            id: 1,
            charge_point_id: "CP001".to_string(),
            evse_id: 1,
            profile_id: 100,
            stack_level: 0,
            purpose: "TxDefaultProfile".to_string(),
            kind: "Absolute".to_string(),
            recurrency_kind: None,
            valid_from: None,
            valid_to: None,
            schedule_json: r#"[{"id":1,"chargingRateUnit":"W"}]"#.to_string(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_value(&profile).unwrap();
        assert_eq!(json["charge_point_id"], "CP001");
        assert_eq!(json["profile_id"], 100);
        assert!(json["is_active"].as_bool().unwrap());
    }
}
