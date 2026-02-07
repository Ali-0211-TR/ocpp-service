//! Health check endpoint

use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

/// Состояние сервиса
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Статус: `ok` — сервис работает нормально
    pub status: String,
    /// Версия OCPP Central System (из Cargo.toml)
    pub version: String,
    /// Время работы сервиса в секундах с момента запуска
    pub uptime_seconds: u64,
}

/// Проверка состояния сервиса
///
/// Возвращает текущий статус, версию и время работы.
/// Не требует авторизации. Используйте для мониторинга доступности.
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Сервис работает нормально", body = HealthResponse)
    )
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: track actual uptime
    })
}
