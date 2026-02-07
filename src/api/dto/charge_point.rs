//! Charge Point DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::{ChargePoint, Connector, ConnectorStatus};

/// Зарядная станция (Charge Point) — полное представление для API
///
/// Станция регистрируется автоматически при первом подключении через OCPP WebSocket.
/// Поле `is_online` отражает текущее WebSocket-соединение.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": "CP001",
    "vendor": "AVT-Company",
    "model": "AVT-Express",
    "serial_number": "avt.001.13.1",
    "firmware_version": "0.9.87",
    "status": "Available",
    "is_online": true,
    "connectors": [{"id": 1, "status": "Available", "error_code": null}],
    "registered_at": "2024-01-15T10:30:00Z",
    "last_heartbeat": "2024-01-15T12:00:00Z"
}))]
pub struct ChargePointDto {
    /// Уникальный идентификатор станции (задаётся станцией при подключении)
    pub id: String,
    /// Производитель оборудования (из BootNotification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    /// Модель станции (из BootNotification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Серийный номер станции
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    /// Версия прошивки
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware_version: Option<String>,
    /// ICCID SIM-карты
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iccid: Option<String>,
    /// IMSI SIM-карты
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imsi: Option<String>,
    /// Текущий статус: Available, Preparing, Charging, SuspendedEV, Finishing, Reserved, Unavailable, Faulted
    pub status: String,
    /// Подключена ли станция по WebSocket прямо сейчас
    pub is_online: bool,
    /// Список коннекторов (разъёмов) станции
    pub connectors: Vec<ConnectorDto>,
    /// Дата первой регистрации станции в системе (UTC, ISO 8601)
    pub registered_at: DateTime<Utc>,
    /// Время последнего Heartbeat от станции (UTC, ISO 8601). null если heartbeat не получен
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat: Option<DateTime<Utc>>,
}

impl ChargePointDto {
    pub fn from_domain(cp: ChargePoint, is_online: bool) -> Self {
        Self {
            id: cp.id,
            vendor: cp.vendor,
            model: cp.model,
            serial_number: cp.serial_number,
            firmware_version: cp.firmware_version,
            iccid: cp.iccid,
            imsi: cp.imsi,
            status: cp.status.to_string(),
            is_online,
            connectors: cp.connectors.into_iter().map(ConnectorDto::from_domain).collect(),
            registered_at: cp.registered_at,
            last_heartbeat: cp.last_heartbeat,
        }
    }
}

/// Коннектор (разъём) зарядной станции
///
/// Каждая станция может иметь один или несколько коннекторов (connector_id ≥ 1).
/// connector_id = 0 зарезервирован для всей станции целиком.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": 1,
    "status": "Available",
    "error_code": null,
    "error_info": null
}))]
pub struct ConnectorDto {
    /// Номер коннектора (1-based). Совпадает с физическим разъёмом на станции
    pub id: u32,
    /// Текущий статус: Available, Preparing, Charging, SuspendedEV, SuspendedEVSE, Finishing, Reserved, Unavailable, Faulted
    pub status: String,
    /// Код ошибки OCPP (NoError, ConnectorLockFailure, HighTemperature, и т.д.). null если ошибки нет
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Дополнительная информация об ошибке от станции
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_info: Option<String>,
}

impl ConnectorDto {
    pub fn from_domain(connector: Connector) -> Self {
        Self {
            id: connector.id,
            status: connector_status_to_string(&connector.status),
            error_code: connector.error_code,
            error_info: connector.info,
        }
    }
}

fn connector_status_to_string(status: &ConnectorStatus) -> String {
    match status {
        ConnectorStatus::Available => "Available",
        ConnectorStatus::Preparing => "Preparing",
        ConnectorStatus::Charging => "Charging",
        ConnectorStatus::SuspendedEV => "SuspendedEV",
        ConnectorStatus::SuspendedEVSE => "SuspendedEVSE",
        ConnectorStatus::Finishing => "Finishing",
        ConnectorStatus::Reserved => "Reserved",
        ConnectorStatus::Unavailable => "Unavailable",
        ConnectorStatus::Faulted => "Faulted",
    }
    .to_string()
}

/// Сводная статистика по зарядным станциям
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "total": 10,
    "online": 8,
    "offline": 2,
    "charging": 3
}))]
pub struct ChargePointStats {
    /// Общее количество зарегистрированных станций
    pub total: u32,
    /// Количество станций, подключённых по WebSocket
    pub online: u32,
    /// Количество неподключённых станций
    pub offline: u32,
    /// Количество станций, на которых идёт зарядка (хотя бы один коннектор в статусе Charging)
    pub charging: u32,
}
