//! Charge Point domain entity

use chrono::{DateTime, Utc};

use super::super::ocpp::OcppVersion;

/// Connector status on a charge point
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectorStatus {
    Available,
    Preparing,
    Charging,
    SuspendedEV,
    SuspendedEVSE,
    Finishing,
    Reserved,
    Unavailable,
    Faulted,
}

impl Default for ConnectorStatus {
    fn default() -> Self {
        Self::Available
    }
}

/// Charge point operational status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChargePointStatus {
    /// Online and communicating
    Online,
    /// Not currently connected
    Offline,
    /// Station is unavailable (error or maintenance)
    Unavailable,
    /// Unknown status (never connected)
    Unknown,
}

impl Default for ChargePointStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for ChargePointStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => write!(f, "Online"),
            Self::Offline => write!(f, "Offline"),
            Self::Unavailable => write!(f, "Unavailable"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl From<&str> for ChargePointStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "online" => Self::Online,
            "offline" => Self::Offline,
            "unavailable" => Self::Unavailable,
            _ => Self::Unknown,
        }
    }
}

impl From<String> for ChargePointStatus {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

/// Connector on a charge point
#[derive(Debug, Clone)]
pub struct Connector {
    pub id: u32,
    pub status: ConnectorStatus,
    pub error_code: Option<String>,
    pub info: Option<String>,
    pub vendor_id: Option<String>,
    pub vendor_error_code: Option<String>,
}

impl Connector {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            status: ConnectorStatus::default(),
            error_code: None,
            info: None,
            vendor_id: None,
            vendor_error_code: None,
        }
    }
}

/// Charge Point entity
#[derive(Debug, Clone)]
pub struct ChargePoint {
    /// Unique identifier
    pub id: String,
    /// Negotiated OCPP protocol version (set on first BootNotification)
    pub ocpp_version: Option<OcppVersion>,
    /// Vendor name
    pub vendor: Option<String>,
    /// Model name
    pub model: Option<String>,
    /// Serial number
    pub serial_number: Option<String>,
    /// Firmware version
    pub firmware_version: Option<String>,
    /// ICCID of the modem
    pub iccid: Option<String>,
    /// IMSI of the modem
    pub imsi: Option<String>,
    /// Meter type
    pub meter_type: Option<String>,
    /// Meter serial number
    pub meter_serial_number: Option<String>,
    /// Registration status
    pub status: ChargePointStatus,
    /// Connectors
    pub connectors: Vec<Connector>,
    /// When the charge point was registered
    pub registered_at: DateTime<Utc>,
    /// Last heartbeat received
    pub last_heartbeat: Option<DateTime<Utc>>,
}

impl ChargePoint {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ocpp_version: None,
            vendor: None,
            model: None,
            serial_number: None,
            firmware_version: None,
            iccid: None,
            imsi: None,
            meter_type: None,
            meter_serial_number: None,
            status: ChargePointStatus::Unknown,
            connectors: Vec::new(),
            registered_at: Utc::now(),
            last_heartbeat: None,
        }
    }

    pub fn set_online(&mut self) {
        self.status = ChargePointStatus::Online;
        self.last_heartbeat = Some(Utc::now());
    }

    pub fn set_offline(&mut self) {
        self.status = ChargePointStatus::Offline;
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Some(Utc::now());
        // Also set status to online when we receive a heartbeat
        if self.status != ChargePointStatus::Online {
            self.status = ChargePointStatus::Online;
        }
    }

    pub fn get_connector(&self, id: u32) -> Option<&Connector> {
        self.connectors.iter().find(|c| c.id == id)
    }

    pub fn get_connector_mut(&mut self, id: u32) -> Option<&mut Connector> {
        self.connectors.iter_mut().find(|c| c.id == id)
    }

    pub fn update_connector_status(&mut self, connector_id: u32, status: ConnectorStatus) {
        if let Some(connector) = self.get_connector_mut(connector_id) {
            connector.status = status;
        } else {
            // Create new connector if it doesn't exist
            let mut connector = Connector::new(connector_id);
            connector.status = status;
            self.connectors.push(connector);
        }
    }

    /// Add a connector with the given ID. Returns false if already exists.
    pub fn add_connector(&mut self, connector_id: u32) -> bool {
        if self.get_connector(connector_id).is_some() {
            return false;
        }
        self.connectors.push(Connector::new(connector_id));
        self.connectors.sort_by_key(|c| c.id);
        true
    }

    /// Remove a connector by ID. Returns false if not found.
    pub fn remove_connector(&mut self, connector_id: u32) -> bool {
        let len_before = self.connectors.len();
        self.connectors.retain(|c| c.id != connector_id);
        self.connectors.len() < len_before
    }

    /// Ensure connectors 0..=num_connectors exist, creating any that are missing.
    /// Connector 0 represents the charge-point itself (OCPP 1.6 convention).
    pub fn ensure_connectors(&mut self, num_connectors: u32) {
        for id in 0..=num_connectors {
            if self.get_connector(id).is_none() {
                self.connectors.push(Connector::new(id));
            }
        }
        self.connectors.sort_by_key(|c| c.id);
    }
}
