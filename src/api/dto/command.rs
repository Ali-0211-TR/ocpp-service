//! Command DTOs for sending commands to charge points

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Запрос на удалённый запуск зарядки (RemoteStartTransaction)
///
/// Отправляет команду на станцию для начала зарядной сессии.
/// Станция должна быть онлайн.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "id_tag": "RFID001",
    "connector_id": 1
}))]
pub struct RemoteStartRequest {
    /// RFID-карта или токен авторизации. Должен существовать в списке IdTags со статусом Accepted
    pub id_tag: String,
    /// Номер коннектора (1-based). Если не указан, станция выберет сама
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<u32>,
}

/// Запрос на удалённую остановку зарядки (RemoteStopTransaction)
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "transaction_id": 123
}))]
pub struct RemoteStopRequest {
    /// ID активной транзакции для остановки. Получить можно из списка активных транзакций
    pub transaction_id: i32,
}

/// Запрос на перезагрузку станции (Reset)
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "type": "Soft"
}))]
pub struct ResetRequest {
    /// Тип перезагрузки: `Soft` (мягкая, дождаться завершения транзакций) или `Hard` (принудительная, немедленная)
    #[serde(rename = "type")]
    pub reset_type: String,
}

/// Запрос на разблокировку коннектора (UnlockConnector)
///
/// Используется когда кабель застрял или нужно принудительно освободить разъём.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "connector_id": 1
}))]
pub struct UnlockConnectorRequest {
    /// Номер коннектора для разблокировки (1-based)
    pub connector_id: u32,
}

/// Запрос на изменение доступности коннектора (ChangeAvailability)
///
/// connector_id = 0 меняет доступность всей станции.
/// Ответ может быть `Accepted`, `Rejected` или `Scheduled`
/// (если идёт активная транзакция — изменение применится после её завершения).
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "connector_id": 0,
    "type": "Operative"
}))]
pub struct ChangeAvailabilityRequest {
    /// Номер коннектора (0 = вся станция, ≥1 = конкретный разъём)
    pub connector_id: u32,
    /// Тип доступности: `Operative` (доступен) или `Inoperative` (недоступен, обслуживание)
    #[serde(rename = "type")]
    pub availability_type: String,
}

/// Запрос на принудительную отправку сообщения (TriggerMessage)
///
/// Запрашивает у станции немедленную отправку указанного сообщения.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "message": "StatusNotification",
    "connector_id": 1
}))]
pub struct TriggerMessageRequest {
    /// Тип сообщения: `BootNotification`, `Heartbeat`, `StatusNotification`, `MeterValues`, `DiagnosticsStatusNotification`, `FirmwareStatusNotification`
    pub message: String,
    /// Номер коннектора (нужен для StatusNotification и MeterValues)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<u32>,
}

/// Ответ на OCPP-команду
///
/// Содержит статус от зарядной станции и описание результата.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "status": "Accepted",
    "message": "Команда принята станцией"
}))]
pub struct CommandResponse {
    /// Статус от станции: `Accepted`, `Rejected`, `NotSupported`, `NotImplemented`, `Scheduled` и др.
    pub status: String,
    /// Описание результата выполнения
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl CommandResponse {
    pub fn accepted() -> Self {
        Self {
            status: "Accepted".to_string(),
            message: Some("Command sent successfully".to_string()),
        }
    }

    pub fn rejected(reason: impl Into<String>) -> Self {
        Self {
            status: "Rejected".to_string(),
            message: Some(reason.into()),
        }
    }

    pub fn from_status(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            message: None,
        }
    }
}

/// Запрос на изменение конфигурации станции (ChangeConfiguration)
///
/// Устанавливает новое значение OCPP-ключа конфигурации.
/// Станция может ответить: `Accepted`, `Rejected`, `RebootRequired`, `NotSupported`.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "key": "HeartbeatInterval",
    "value": "300"
}))]
pub struct ChangeConfigurationRequest {
    /// Ключ конфигурации OCPP.
    /// Стандартные ключи: `HeartbeatInterval`, `MeterValueSampleInterval`,
    /// `ConnectionTimeOut`, `ClockAlignedDataInterval`, `LocalPreAuthorize`,
    /// `AuthorizeRemoteTxRequests`, `NumberOfConnectors`
    pub key: String,
    /// Новое значение ключа (строка — станция сама интерпретирует тип)
    pub value: String,
}

/// Запрос на произвольный обмен данными (DataTransfer)
///
/// Вендор-специфичная передача данных на станцию.
/// Используется для проприетарных расширений OCPP-протокола.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "vendor_id": "TexnoUZ",
    "message_id": "GetCustomData",
    "data": "{\"param\": \"value\"}"
}))]
pub struct DataTransferRequest {
    /// Идентификатор вендора (производителя станции)
    pub vendor_id: String,
    /// Идентификатор сообщения (опционально)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// Произвольные данные (обычно JSON-строка)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Ответ DataTransfer от станции
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
    "status": "Accepted",
    "data": "{\"firmware\": \"1.2.3\"}"
}))]
pub struct DataTransferResponse {
    /// Статус: `Accepted`, `Rejected`, `UnknownMessageId`, `UnknownVendorId`
    pub status: String,
    /// Данные от станции (если есть)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Ответ GetLocalListVersion
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
    "list_version": 5
}))]
pub struct LocalListVersionResponse {
    /// Версия локального списка авторизации.
    /// `-1` = не поддерживается, `0` = список пуст, `>0` = текущая версия
    pub list_version: i32,
}
