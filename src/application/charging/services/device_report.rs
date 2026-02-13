//! In-memory device report store for NotifyReport aggregation.
//!
//! When a CSMS sends `GetBaseReport`, the charge point may reply with
//! multiple `NotifyReport` messages (`tbc = true` until the last one).
//! This store collects all parts keyed by `(charge_point_id, request_id)`
//! and provides a way to retrieve the full assembled report.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single variable report entry (flattened from ReportDataType).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportVariable {
    /// Component name (e.g. "EVSE", "Connector", "ChargingStation").
    pub component: String,
    /// Component instance, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_instance: Option<String>,
    /// EVSE ID, if the variable is scoped to an EVSE.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evse_id: Option<i32>,
    /// Connector ID within the EVSE, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<i32>,
    /// Variable name.
    pub variable: String,
    /// Variable instance, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_instance: Option<String>,
    /// Attribute values (type → value).
    pub attributes: Vec<VariableAttributeEntry>,
    /// Data type (string, integer, boolean, etc.) from VariableCharacteristics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
    /// Unit of measurement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// A single variable attribute entry.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VariableAttributeEntry {
    /// Attribute type: Actual, Target, MinSet, MaxSet.
    #[serde(rename = "type")]
    pub attr_type: String,
    /// The value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Mutability: ReadOnly, WriteOnly, ReadWrite.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutability: Option<String>,
}

/// Aggregated device report for a single GetBaseReport request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeviceReport {
    /// The charge point ID.
    pub charge_point_id: String,
    /// The request ID from GetBaseReport.
    pub request_id: i32,
    /// When the first report part was received.
    pub started_at: DateTime<Utc>,
    /// When the last report part was received (tbc=false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Whether the report is still being received.
    pub in_progress: bool,
    /// Total number of parts received.
    pub parts_received: i32,
    /// All variables reported.
    pub variables: Vec<ReportVariable>,
}

/// Key for the report store.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ReportKey {
    charge_point_id: String,
    request_id: i32,
}

/// Thread-safe in-memory store for device reports.
#[derive(Debug, Clone)]
pub struct DeviceReportStore {
    reports: Arc<DashMap<ReportKey, DeviceReport>>,
    /// Also keep a mapping of charge_point_id → latest request_id
    /// for easy lookup.
    latest: Arc<DashMap<String, i32>>,
}

impl Default for DeviceReportStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceReportStore {
    pub fn new() -> Self {
        Self {
            reports: Arc::new(DashMap::new()),
            latest: Arc::new(DashMap::new()),
        }
    }

    /// Initialize a new report when GetBaseReport is sent.
    pub fn init_report(&self, charge_point_id: &str, request_id: i32) {
        let key = ReportKey {
            charge_point_id: charge_point_id.to_string(),
            request_id,
        };
        self.reports.insert(
            key,
            DeviceReport {
                charge_point_id: charge_point_id.to_string(),
                request_id,
                started_at: Utc::now(),
                completed_at: None,
                in_progress: true,
                parts_received: 0,
                variables: Vec::new(),
            },
        );
        self.latest
            .insert(charge_point_id.to_string(), request_id);
    }

    /// Append variables from a NotifyReport message.
    ///
    /// `tbc` = true means more parts are coming; false means this is the last.
    pub fn append_report(
        &self,
        charge_point_id: &str,
        request_id: i32,
        variables: Vec<ReportVariable>,
        tbc: bool,
    ) {
        let key = ReportKey {
            charge_point_id: charge_point_id.to_string(),
            request_id,
        };

        // If we don't have a report for this key, create one on the fly
        // (in case the init was missed).
        self.reports
            .entry(key)
            .and_modify(|report| {
                report.parts_received += 1;
                report.variables.extend(variables.clone());
                if !tbc {
                    report.in_progress = false;
                    report.completed_at = Some(Utc::now());
                }
            })
            .or_insert_with(|| DeviceReport {
                charge_point_id: charge_point_id.to_string(),
                request_id,
                started_at: Utc::now(),
                completed_at: if tbc { None } else { Some(Utc::now()) },
                in_progress: tbc,
                parts_received: 1,
                variables,
            });

        self.latest
            .entry(charge_point_id.to_string())
            .or_insert(request_id);
    }

    /// Get the latest report for a charge point.
    pub fn get_latest_report(&self, charge_point_id: &str) -> Option<DeviceReport> {
        let request_id = self.latest.get(charge_point_id)?;
        let key = ReportKey {
            charge_point_id: charge_point_id.to_string(),
            request_id: *request_id,
        };
        self.reports.get(&key).map(|r| r.clone())
    }

    /// Get a specific report by charge_point_id and request_id.
    pub fn get_report(&self, charge_point_id: &str, request_id: i32) -> Option<DeviceReport> {
        let key = ReportKey {
            charge_point_id: charge_point_id.to_string(),
            request_id,
        };
        self.reports.get(&key).map(|r| r.clone())
    }
}

/// Shared device report store.
pub type SharedDeviceReportStore = Arc<DeviceReportStore>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_report_creates_empty() {
        let store = DeviceReportStore::new();
        store.init_report("CP001", 1);

        let report = store.get_latest_report("CP001").unwrap();
        assert_eq!(report.charge_point_id, "CP001");
        assert_eq!(report.request_id, 1);
        assert!(report.in_progress);
        assert_eq!(report.parts_received, 0);
        assert!(report.variables.is_empty());
    }

    #[test]
    fn append_single_part_completes() {
        let store = DeviceReportStore::new();
        store.init_report("CP001", 1);

        let vars = vec![ReportVariable {
            component: "EVSE".to_string(),
            component_instance: None,
            evse_id: Some(1),
            connector_id: None,
            variable: "Power".to_string(),
            variable_instance: None,
            attributes: vec![VariableAttributeEntry {
                attr_type: "Actual".to_string(),
                value: Some("22000".to_string()),
                mutability: Some("ReadOnly".to_string()),
            }],
            data_type: Some("integer".to_string()),
            unit: Some("W".to_string()),
        }];

        store.append_report("CP001", 1, vars, false);

        let report = store.get_latest_report("CP001").unwrap();
        assert!(!report.in_progress);
        assert_eq!(report.parts_received, 1);
        assert_eq!(report.variables.len(), 1);
        assert!(report.completed_at.is_some());
    }

    #[test]
    fn append_multi_part_report() {
        let store = DeviceReportStore::new();
        store.init_report("CP001", 42);

        let part1 = vec![ReportVariable {
            component: "EVSE".to_string(),
            component_instance: None,
            evse_id: Some(1),
            connector_id: None,
            variable: "Power".to_string(),
            variable_instance: None,
            attributes: vec![],
            data_type: None,
            unit: None,
        }];
        store.append_report("CP001", 42, part1, true);

        let report = store.get_latest_report("CP001").unwrap();
        assert!(report.in_progress);
        assert_eq!(report.parts_received, 1);

        let part2 = vec![ReportVariable {
            component: "Connector".to_string(),
            component_instance: None,
            evse_id: Some(1),
            connector_id: Some(1),
            variable: "AvailabilityState".to_string(),
            variable_instance: None,
            attributes: vec![],
            data_type: None,
            unit: None,
        }];
        store.append_report("CP001", 42, part2, false);

        let report = store.get_latest_report("CP001").unwrap();
        assert!(!report.in_progress);
        assert_eq!(report.parts_received, 2);
        assert_eq!(report.variables.len(), 2);
    }

    #[test]
    fn get_report_by_request_id() {
        let store = DeviceReportStore::new();
        store.init_report("CP001", 1);
        store.init_report("CP001", 2);

        store.append_report("CP001", 1, vec![], false);
        store.append_report("CP001", 2, vec![], false);

        let r1 = store.get_report("CP001", 1).unwrap();
        assert_eq!(r1.request_id, 1);

        let r2 = store.get_report("CP001", 2).unwrap();
        assert_eq!(r2.request_id, 2);
    }

    #[test]
    fn latest_report_tracks_most_recent() {
        let store = DeviceReportStore::new();
        store.init_report("CP001", 1);
        store.init_report("CP001", 5);

        let latest = store.get_latest_report("CP001").unwrap();
        assert_eq!(latest.request_id, 5);
    }

    #[test]
    fn missing_charge_point_returns_none() {
        let store = DeviceReportStore::new();
        assert!(store.get_latest_report("UNKNOWN").is_none());
        assert!(store.get_report("UNKNOWN", 1).is_none());
    }
}
