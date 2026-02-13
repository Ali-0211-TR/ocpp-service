//! V201 ReportChargingProfiles handler
//!
//! Receives charging profile reports from charge points in response
//! to a GetChargingProfiles command. Logs the reported profiles.

use rust_ocpp::v2_0_1::messages::report_charging_profiles::{
    ReportChargingProfilesRequest, ReportChargingProfilesResponse,
};
use serde_json::Value;
use tracing::{error, info};

use crate::application::OcppHandlerV201;

pub async fn handle_report_charging_profiles(
    handler: &OcppHandlerV201,
    payload: &Value,
) -> Value {
    let req: ReportChargingProfilesRequest = match serde_json::from_value(payload.clone()) {
        Ok(r) => r,
        Err(e) => {
            error!(
                charge_point_id = handler.charge_point_id.as_str(),
                error = %e,
                "V201: Failed to parse ReportChargingProfiles"
            );
            return serde_json::to_value(ReportChargingProfilesResponse {}).unwrap_or_default();
        }
    };

    let tbc = req.tbc.unwrap_or(false);
    let profile_count = req.charging_profile.len();

    info!(
        charge_point_id = handler.charge_point_id.as_str(),
        request_id = req.request_id,
        evse_id = req.evse_id,
        charging_limit_source = ?req.charging_limit_source,
        tbc,
        profiles = profile_count,
        "V201 ReportChargingProfiles received ({} profile(s){})",
        profile_count,
        if tbc { ", more coming" } else { "" }
    );

    for profile in &req.charging_profile {
        info!(
            charge_point_id = handler.charge_point_id.as_str(),
            profile_id = profile.id,
            stack_level = profile.stack_level,
            purpose = ?profile.charging_profile_purpose,
            kind = ?profile.charging_profile_kind,
            schedules = profile.charging_schedule.len(),
            "  Profile id={}, purpose={:?}, stack_level={}, schedules={}",
            profile.id,
            profile.charging_profile_purpose,
            profile.stack_level,
            profile.charging_schedule.len()
        );
    }

    serde_json::to_value(ReportChargingProfilesResponse {}).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_charging_profiles_response_shape() {
        let resp = ReportChargingProfilesResponse {};
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json, serde_json::json!({}));
    }
}
