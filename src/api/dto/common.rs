//! Common API DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Стандартная обёртка ответа API
///
/// Все REST-эндпоинты возвращают данные в этой обёртке.
/// При успехе: `{"success": true, "data": {...}}`,
/// при ошибке: `{"success": false, "error": "описание"}`.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// `true` если запрос выполнен успешно
    pub success: bool,
    /// Полезная нагрузка (данные). `null` при ошибке
    pub data: Option<T>,
    /// Описание ошибки. `null` при успехе
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Empty response for operations without return data
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EmptyData {}

/// Параметры пагинации для запросов со списками
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PaginationParams {
    /// Номер страницы (начиная с 1). По умолчанию: 1
    #[serde(default = "default_page")]
    pub page: u32,
    /// Количество элементов на странице (1–100). По умолчанию: 50
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_page() -> u32 {
    1
}

fn default_limit() -> u32 {
    50
}

/// Ответ с пагинацией
///
/// Содержит срез данных и метаинформацию о странице.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// Массив элементов на текущей странице
    pub items: Vec<T>,
    /// Общее количество элементов (по всем страницам)
    pub total: u64,
    /// Текущая страница (1-based)
    pub page: u32,
    /// Размер страницы
    pub limit: u32,
    /// Общее количество страниц
    pub total_pages: u32,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: u64, page: u32, limit: u32) -> Self {
        let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;
        Self {
            items,
            total,
            page,
            limit,
            total_pages,
        }
    }
}
