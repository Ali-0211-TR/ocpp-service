//! Analytics API handlers
//!
//! All endpoints use SeaORM entity queries directly for efficient SQL aggregation.
//! They avoid loading entire result sets into memory when possible.

use axum::extract::{Query, State};
use axum::Json;
use axum::http::StatusCode;
use chrono::{Datelike, Duration, NaiveTime, Timelike, Utc};
use sea_orm::prelude::*;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

use super::dto::*;
use crate::application::charging::session::SharedSessionRegistry;
use crate::infrastructure::database::entities::{
    charge_point as cp_entity, transaction as tx_entity,
};
use crate::interfaces::http::common::ApiResponse;

/// Analytics handler state.
#[derive(Clone)]
pub struct AnalyticsState {
    pub db: DatabaseConnection,
    pub session_registry: SharedSessionRegistry,
}

// ── Query params ───────────────────────────────────────────────

/// Period query parameter for revenue / energy endpoints.
#[derive(Debug, serde::Deserialize)]
pub struct PeriodParams {
    /// Granularity: "day", "week", or "month". Defaults to "day".
    pub period: Option<String>,
}

/// Optional days-back param for peak hours.
#[derive(Debug, serde::Deserialize)]
pub struct PeakHoursParams {
    /// Number of days to look back (default 30).
    pub days: Option<u32>,
}

// ── 1. Summary ─────────────────────────────────────────────────

/// Overall dashboard summary.
#[utoipa::path(
    get,
    path = "/api/v1/analytics/summary",
    tag = "Analytics",
    security(("bearer_auth" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "Dashboard summary", body = ApiResponse<AnalyticsSummary>)
    )
)]
pub async fn analytics_summary(
    State(state): State<AnalyticsState>,
) -> Result<Json<ApiResponse<AnalyticsSummary>>, (StatusCode, Json<ApiResponse<AnalyticsSummary>>)>
{
    let db = &state.db;
    let now = Utc::now();

    // -- stations --
    let all_cps: Vec<cp_entity::Model> = cp_entity::Entity::find()
        .all(db)
        .await
        .unwrap_or_default();

    let total_stations = all_cps.len() as u64;
    let connected_ids = state.session_registry.connected_ids();
    let stations_online = connected_ids.len() as u64;
    let stations_offline = total_stations.saturating_sub(stations_online);

    // -- active transactions --
    let active_transactions = tx_entity::Entity::find()
        .filter(tx_entity::Column::Status.eq("Active"))
        .count(db)
        .await
        .unwrap_or(0);

    // -- today boundaries (UTC) --
    let today_start = now
        .date_naive()
        .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .and_utc();

    // -- month boundaries --
    let month_start = now
        .date_naive()
        .with_day(1)
        .unwrap_or(now.date_naive())
        .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .and_utc();

    // -- today's completed transactions --
    let today_txs: Vec<tx_entity::Model> = tx_entity::Entity::find()
        .filter(
            Condition::all()
                .add(tx_entity::Column::Status.eq("Completed"))
                .add(tx_entity::Column::StoppedAt.gte(today_start)),
        )
        .all(db)
        .await
        .unwrap_or_default();

    let transactions_today = today_txs.len() as u64;
    let revenue_today: i64 = today_txs.iter().filter_map(|t| t.total_cost).map(|c| c as i64).sum();
    let energy_today_wh: i64 = today_txs
        .iter()
        .filter_map(|t| t.energy_consumed)
        .map(|e| e as i64)
        .sum();

    // -- month totals --
    let month_txs: Vec<tx_entity::Model> = tx_entity::Entity::find()
        .filter(
            Condition::all()
                .add(tx_entity::Column::Status.eq("Completed"))
                .add(tx_entity::Column::StoppedAt.gte(month_start)),
        )
        .all(db)
        .await
        .unwrap_or_default();

    let revenue_month: i64 = month_txs.iter().filter_map(|t| t.total_cost).map(|c| c as i64).sum();
    let energy_month_wh: i64 = month_txs
        .iter()
        .filter_map(|t| t.energy_consumed)
        .map(|e| e as i64)
        .sum();

    // currency from most recent billed tx
    let currency = month_txs
        .iter()
        .rev()
        .find_map(|t| t.currency.clone())
        .unwrap_or_else(|| "UZS".to_string());

    Ok(Json(ApiResponse::success(AnalyticsSummary {
        total_stations,
        stations_online,
        stations_offline,
        active_transactions,
        transactions_today,
        revenue_today,
        revenue_month,
        energy_today_wh,
        energy_month_wh,
        currency,
    })))
}

// ── 2. Revenue ─────────────────────────────────────────────────

/// Revenue breakdown by day / week / month.
#[utoipa::path(
    get,
    path = "/api/v1/analytics/revenue",
    tag = "Analytics",
    params(
        ("period" = Option<String>, Query, description = "Granularity: day, week, month (default: day)")
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "Revenue breakdown", body = ApiResponse<RevenueResponse>)
    )
)]
pub async fn analytics_revenue(
    State(state): State<AnalyticsState>,
    Query(params): Query<PeriodParams>,
) -> Result<Json<ApiResponse<RevenueResponse>>, (StatusCode, Json<ApiResponse<RevenueResponse>>)> {
    let db = &state.db;
    let granularity = params.period.unwrap_or_else(|| "day".to_string());

    let lookback_days: i64 = match granularity.as_str() {
        "month" => 365,
        "week" => 90,
        _ => 30, // day
    };

    let since = Utc::now() - Duration::days(lookback_days);

    let txs: Vec<tx_entity::Model> = tx_entity::Entity::find()
        .filter(
            Condition::all()
                .add(tx_entity::Column::Status.eq("Completed"))
                .add(tx_entity::Column::StoppedAt.gte(since)),
        )
        .order_by_asc(tx_entity::Column::StoppedAt)
        .all(db)
        .await
        .unwrap_or_default();

    let mut bucket_map: std::collections::BTreeMap<String, (i64, u64)> =
        std::collections::BTreeMap::new();

    for tx in &txs {
        if let Some(stopped) = tx.stopped_at {
            let key = bucket_key(&granularity, stopped);
            let entry = bucket_map.entry(key).or_insert((0, 0));
            entry.0 += tx.total_cost.unwrap_or(0) as i64;
            entry.1 += 1;
        }
    }

    let mut total_revenue = 0i64;
    let buckets: Vec<RevenueBucket> = bucket_map
        .into_iter()
        .map(|(period, (revenue, count))| {
            total_revenue += revenue;
            RevenueBucket {
                period,
                revenue,
                transaction_count: count,
            }
        })
        .collect();

    let currency = txs
        .iter()
        .rev()
        .find_map(|t| t.currency.clone())
        .unwrap_or_else(|| "UZS".to_string());

    Ok(Json(ApiResponse::success(RevenueResponse {
        granularity,
        buckets,
        total_revenue,
        currency,
    })))
}

// ── 3. Energy ──────────────────────────────────────────────────

/// Energy consumption breakdown by day / week / month.
#[utoipa::path(
    get,
    path = "/api/v1/analytics/energy",
    tag = "Analytics",
    params(
        ("period" = Option<String>, Query, description = "Granularity: day, week, month (default: day)")
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "Energy breakdown", body = ApiResponse<EnergyResponse>)
    )
)]
pub async fn analytics_energy(
    State(state): State<AnalyticsState>,
    Query(params): Query<PeriodParams>,
) -> Result<Json<ApiResponse<EnergyResponse>>, (StatusCode, Json<ApiResponse<EnergyResponse>>)> {
    let db = &state.db;
    let granularity = params.period.unwrap_or_else(|| "day".to_string());

    let lookback_days: i64 = match granularity.as_str() {
        "month" => 365,
        "week" => 90,
        _ => 30,
    };

    let since = Utc::now() - Duration::days(lookback_days);

    let txs: Vec<tx_entity::Model> = tx_entity::Entity::find()
        .filter(
            Condition::all()
                .add(tx_entity::Column::Status.eq("Completed"))
                .add(tx_entity::Column::StoppedAt.gte(since)),
        )
        .order_by_asc(tx_entity::Column::StoppedAt)
        .all(db)
        .await
        .unwrap_or_default();

    let mut bucket_map: std::collections::BTreeMap<String, (i64, u64)> =
        std::collections::BTreeMap::new();

    for tx in &txs {
        if let Some(stopped) = tx.stopped_at {
            let key = bucket_key(&granularity, stopped);
            let entry = bucket_map.entry(key).or_insert((0, 0));
            entry.0 += tx.energy_consumed.unwrap_or(0) as i64;
            entry.1 += 1;
        }
    }

    let mut total_energy_wh = 0i64;
    let buckets: Vec<EnergyBucket> = bucket_map
        .into_iter()
        .map(|(period, (energy, count))| {
            total_energy_wh += energy;
            EnergyBucket {
                period,
                energy_wh: energy,
                transaction_count: count,
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(EnergyResponse {
        granularity,
        buckets,
        total_energy_wh,
    })))
}

// ── 4. Peak Hours ──────────────────────────────────────────────

/// Peak load hours — number of transactions and energy per hour of day.
#[utoipa::path(
    get,
    path = "/api/v1/analytics/peak-hours",
    tag = "Analytics",
    params(
        ("days" = Option<u32>, Query, description = "Look-back period in days (default 30)")
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "Peak hours breakdown", body = ApiResponse<PeakHoursResponse>)
    )
)]
pub async fn analytics_peak_hours(
    State(state): State<AnalyticsState>,
    Query(params): Query<PeakHoursParams>,
) -> Result<Json<ApiResponse<PeakHoursResponse>>, (StatusCode, Json<ApiResponse<PeakHoursResponse>>)>
{
    let db = &state.db;
    let days = params.days.unwrap_or(30) as i64;
    let since = Utc::now() - Duration::days(days);

    let txs: Vec<tx_entity::Model> = tx_entity::Entity::find()
        .filter(tx_entity::Column::StartedAt.gte(since))
        .all(db)
        .await
        .unwrap_or_default();

    // Aggregate by hour (0–23)
    let mut counts = [0u64; 24];
    let mut energy = [0i64; 24];

    for tx in &txs {
        let hour = tx.started_at.hour() as usize;
        if hour < 24 {
            counts[hour] += 1;
            energy[hour] += tx.energy_consumed.unwrap_or(0) as i64;
        }
    }

    let busiest_hour = counts
        .iter()
        .enumerate()
        .max_by_key(|(_, c)| *c)
        .map(|(h, _)| h as u8)
        .unwrap_or(0);

    let hours: Vec<PeakHourEntry> = (0..24)
        .map(|h| PeakHourEntry {
            hour: h as u8,
            transaction_count: counts[h],
            energy_wh: energy[h],
        })
        .collect();

    Ok(Json(ApiResponse::success(PeakHoursResponse {
        hours,
        busiest_hour,
    })))
}

// ── 5. Station Uptime ──────────────────────────────────────────

/// Per-station uptime and aggregate statistics.
#[utoipa::path(
    get,
    path = "/api/v1/analytics/station-uptime",
    tag = "Analytics",
    security(("bearer_auth" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "Station uptime data", body = ApiResponse<StationUptimeResponse>)
    )
)]
pub async fn analytics_station_uptime(
    State(state): State<AnalyticsState>,
) -> Result<
    Json<ApiResponse<StationUptimeResponse>>,
    (StatusCode, Json<ApiResponse<StationUptimeResponse>>),
> {
    let db = &state.db;
    let now = Utc::now();

    // Fetch all charge points
    let all_cps: Vec<cp_entity::Model> = cp_entity::Entity::find()
        .order_by_asc(cp_entity::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();

    // Fetch all completed transactions for aggregation
    let all_txs: Vec<tx_entity::Model> = tx_entity::Entity::find()
        .filter(tx_entity::Column::Status.eq("Completed"))
        .all(db)
        .await
        .unwrap_or_default();

    // Build per-CP tx aggregates
    let mut tx_agg: std::collections::HashMap<String, (u64, i64)> =
        std::collections::HashMap::new();
    for tx in &all_txs {
        let entry = tx_agg
            .entry(tx.charge_point_id.clone())
            .or_insert((0, 0));
        entry.0 += 1;
        entry.1 += tx.energy_consumed.unwrap_or(0) as i64;
    }

    let connected_ids = state.session_registry.connected_ids();

    let stations: Vec<StationUptimeEntry> = all_cps
        .into_iter()
        .map(|cp| {
            let is_connected = connected_ids.contains(&cp.id);
            let seconds_since_heartbeat = cp.last_heartbeat.map(|hb| (now - hb).num_seconds());
            let (total_tx, total_energy) =
                tx_agg.get(&cp.id).copied().unwrap_or((0, 0));

            StationUptimeEntry {
                charge_point_id: cp.id,
                vendor: cp.vendor,
                model: cp.model,
                status: cp.status,
                is_connected,
                last_heartbeat: cp.last_heartbeat.map(|dt| dt.to_rfc3339()),
                seconds_since_heartbeat,
                total_transactions: total_tx,
                total_energy_wh: total_energy,
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(StationUptimeResponse { stations })))
}

// ── Helpers ────────────────────────────────────────────────────

/// Produce a human-readable bucket key based on granularity.
fn bucket_key(granularity: &str, dt: DateTimeUtc) -> String {
    match granularity {
        "month" => format!("{:04}-{:02}", dt.year(), dt.month()),
        "week" => {
            let iso = dt.iso_week();
            format!("{:04}-W{:02}", iso.year(), iso.week())
        }
        _ => format!("{}", dt.format("%Y-%m-%d")),
    }
}
