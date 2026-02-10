//! Transaction DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::{Transaction, TransactionStatus};

/// Зарядная сессия (транзакция)
///
/// Создаётся автоматически при получении StartTransaction от станции.
/// Завершается при StopTransaction или RemoteStopTransaction.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": 1,
    "charge_point_id": "CP001",
    "connector_id": 1,
    "id_tag": "RFID001",
    "meter_start": 10000,
    "meter_stop": 15000,
    "energy_consumed_wh": 5000,
    "status": "Completed",
    "started_at": "2024-01-15T10:00:00Z",
    "stopped_at": "2024-01-15T12:00:00Z",
    "stop_reason": "Local",
    "last_meter_value": 15000,
    "current_power_w": 7400.0,
    "current_soc": 65,
    "limit_type": "energy",
    "limit_value": 20.0
}))]
pub struct TransactionDto {
    /// Уникальный ID транзакции (автоинкремент)
    pub id: i32,
    /// ID зарядной станции
    pub charge_point_id: String,
    /// Номер коннектора (1-based)
    pub connector_id: u32,
    /// RFID-карта/токен, которым была начата сессия
    pub id_tag: String,
    /// Показания счётчика на начало сессии (Вт·ч)
    pub meter_start: i32,
    /// Показания счётчика на конец сессии (Вт·ч). null если сессия активна
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_stop: Option<i32>,
    /// Потреблённая энергия в Вт·ч (meter_stop - meter_start). null если сессия активна
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_consumed_wh: Option<i32>,
    /// Статус: `Active` (зарядка идёт), `Completed` (завершена), `Failed` (ошибка)
    pub status: String,
    /// Время начала зарядки (UTC, ISO 8601)
    pub started_at: DateTime<Utc>,
    /// Время окончания зарядки (UTC, ISO 8601). null если сессия активна
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<DateTime<Utc>>,
    /// Причина остановки: `Local`, `Remote`, `EmergencyStop`, `EVDisconnected`, `PowerLoss` и др.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    // Live meter data
    /// Последнее показание счётчика (Вт·ч). Обновляется в реальном времени при зарядке
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_meter_value: Option<i32>,
    /// Текущая мощность зарядки (Вт). null если данные недоступны
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_power_w: Option<f64>,
    /// Текущий уровень заряда батареи (%). null если станция не отправляет SoC
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_soc: Option<i32>,
    /// Время последнего обновления данных счётчика
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_meter_update: Option<DateTime<Utc>>,
    // Charging limits
    /// Тип лимита: "energy" (кВт·ч), "amount" (сумма), "soc" (%)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_type: Option<String>,
    /// Значение лимита
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_value: Option<f64>,
}

impl TransactionDto {
    pub fn from_domain(tx: Transaction) -> Self {
        let energy = tx.energy_consumed().or_else(|| tx.live_energy_consumed());
        Self {
            id: tx.id,
            charge_point_id: tx.charge_point_id,
            connector_id: tx.connector_id,
            id_tag: tx.id_tag,
            meter_start: tx.meter_start,
            meter_stop: tx.meter_stop,
            energy_consumed_wh: energy,
            status: transaction_status_to_string(&tx.status),
            started_at: tx.started_at,
            stopped_at: tx.stopped_at,
            stop_reason: tx.stop_reason,
            last_meter_value: tx.last_meter_value,
            current_power_w: tx.current_power_w,
            current_soc: tx.current_soc,
            last_meter_update: tx.last_meter_update,
            limit_type: tx.limit_type.as_ref().map(|lt| lt.as_str().to_string()),
            limit_value: tx.limit_value,
        }
    }
}

fn transaction_status_to_string(status: &TransactionStatus) -> String {
    match status {
        TransactionStatus::Active => "Active",
        TransactionStatus::Completed => "Completed",
        TransactionStatus::Failed => "Failed",
    }
    .to_string()
}

/// Фильтры для списка транзакций
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct TransactionFilter {
    /// Фильтр по ID зарядной станции
    pub charge_point_id: Option<String>,
    /// Фильтр по статусу: `active`, `completed`, `failed`
    pub status: Option<String>,
    /// Начальная дата фильтра (ISO 8601, напр. `2024-01-01T00:00:00Z`)
    pub from_date: Option<DateTime<Utc>>,
    /// Конечная дата фильтра (ISO 8601)
    pub to_date: Option<DateTime<Utc>>,
}
