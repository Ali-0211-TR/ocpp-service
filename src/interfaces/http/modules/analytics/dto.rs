//! Analytics API data transfer objects

use serde::Serialize;
use utoipa::ToSchema;

// ── Summary ────────────────────────────────────────────────────

/// Overall dashboard summary.
#[derive(Debug, Serialize, ToSchema)]
pub struct AnalyticsSummary {
    /// Total number of registered stations.
    pub total_stations: u64,
    /// Stations currently online.
    pub stations_online: u64,
    /// Stations currently offline.
    pub stations_offline: u64,
    /// Active (in-progress) transactions right now.
    pub active_transactions: u64,
    /// Completed transactions today.
    pub transactions_today: u64,
    /// Revenue today in smallest currency unit (e.g. cents/tiyin).
    pub revenue_today: i64,
    /// Revenue this month in smallest currency unit.
    pub revenue_month: i64,
    /// Total energy consumed today in Wh.
    pub energy_today_wh: i64,
    /// Total energy consumed this month in Wh.
    pub energy_month_wh: i64,
    /// Currency code (from most recent transaction, or "UZS").
    pub currency: String,
}

// ── Revenue ────────────────────────────────────────────────────

/// Revenue data point (one per day/week/month bucket).
#[derive(Debug, Serialize, ToSchema)]
pub struct RevenueBucket {
    /// Bucket label (ISO date, ISO week, or YYYY-MM).
    pub period: String,
    /// Total revenue in smallest currency unit.
    pub revenue: i64,
    /// Number of completed transactions.
    pub transaction_count: u64,
}

/// Revenue breakdown response.
#[derive(Debug, Serialize, ToSchema)]
pub struct RevenueResponse {
    /// The requested period granularity ("day", "week", "month").
    pub granularity: String,
    pub buckets: Vec<RevenueBucket>,
    /// Sum of all buckets.
    pub total_revenue: i64,
    pub currency: String,
}

// ── Energy ─────────────────────────────────────────────────────

/// Energy data point.
#[derive(Debug, Serialize, ToSchema)]
pub struct EnergyBucket {
    /// Bucket label.
    pub period: String,
    /// Energy consumed in Wh.
    pub energy_wh: i64,
    /// Number of transactions.
    pub transaction_count: u64,
}

/// Energy breakdown response.
#[derive(Debug, Serialize, ToSchema)]
pub struct EnergyResponse {
    pub granularity: String,
    pub buckets: Vec<EnergyBucket>,
    /// Sum of all buckets in Wh.
    pub total_energy_wh: i64,
}

// ── Peak Hours ─────────────────────────────────────────────────

/// Peak-hour entry (0–23).
#[derive(Debug, Serialize, ToSchema)]
pub struct PeakHourEntry {
    /// Hour of day (0–23).
    pub hour: u8,
    /// Number of transactions that started in this hour.
    pub transaction_count: u64,
    /// Total energy consumed in this hour (Wh).
    pub energy_wh: i64,
}

/// Peak hours response.
#[derive(Debug, Serialize, ToSchema)]
pub struct PeakHoursResponse {
    /// 24 entries, one per hour.
    pub hours: Vec<PeakHourEntry>,
    /// The busiest hour (0–23).
    pub busiest_hour: u8,
}

// ── Station Uptime ─────────────────────────────────────────────

/// Per-station uptime entry.
#[derive(Debug, Serialize, ToSchema)]
pub struct StationUptimeEntry {
    pub charge_point_id: String,
    pub vendor: String,
    pub model: String,
    /// Current status ("Online" / "Offline" / "Unavailable" / "Unknown").
    pub status: String,
    /// Whether the station is currently connected via WebSocket.
    pub is_connected: bool,
    /// Last heartbeat as ISO 8601 (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat: Option<String>,
    /// Seconds since last heartbeat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds_since_heartbeat: Option<i64>,
    /// Total completed transactions.
    pub total_transactions: u64,
    /// Total energy delivered (Wh).
    pub total_energy_wh: i64,
}

/// Station uptime response.
#[derive(Debug, Serialize, ToSchema)]
pub struct StationUptimeResponse {
    pub stations: Vec<StationUptimeEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_summary_serialization() {
        let summary = AnalyticsSummary {
            total_stations: 10,
            stations_online: 7,
            stations_offline: 3,
            active_transactions: 2,
            transactions_today: 15,
            revenue_today: 150_000,
            revenue_month: 3_500_000,
            energy_today_wh: 120_000,
            energy_month_wh: 2_800_000,
            currency: "UZS".to_string(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_stations\":10"));
        assert!(json.contains("\"stations_online\":7"));
    }

    #[test]
    fn test_peak_hours_response() {
        let resp = PeakHoursResponse {
            hours: vec![PeakHourEntry {
                hour: 14,
                transaction_count: 42,
                energy_wh: 350_000,
            }],
            busiest_hour: 14,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"busiest_hour\":14"));
    }
}
