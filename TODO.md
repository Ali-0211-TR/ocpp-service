# TODO — Texnouz OCPP Central System

> Остаточные задачи и рекомендации для доведения до production-ready состояния.
> Сгенерировано: 2026-02-12

---

## 🔴 Критические (безопасность / стабильность)

### 1. Аутентификация WebSocket-подключений зарядных станций
- **Файл:** `src/interfaces/ws/ocpp_server.rs` → `handle_connection()`
- **Проблема:** Любое устройство может подключиться по `ws://<host>:9000/<charge_point_id>` без какой-либо проверки. Нет ни токена, ни API-ключа, ни whitelist-а.
- **Решение:**
  - Добавить проверку `Authorization` header или query-параметра `?token=...` при WebSocket upgrade
  - Или завести whitelist допустимых `charge_point_id` в БД (таблица `charge_points`) и отклонять неизвестные
  - Как минимум — проверка через Basic Auth (login:password в URL) или OCPP SecurityProfile
- **Приоритет:** 🔴 Высокий — без этого любой может имитировать станцию

### 2. CORS — ограничить allowed origins
- **Файл:** `src/interfaces/http/router.rs` → `CorsLayer`
- **Проблема:** Текущая настройка: `CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)` — полностью открыт для любого домена.
- **Решение:**
  - Добавить секцию `[cors]` в `config.rs`:
    ```toml
    [cors]
    allowed_origins = ["https://your-frontend.com", "http://localhost:3000"]
    ```
  - В `router.rs` использовать `AllowOrigin::list()` вместо `Any`
  - Для dev-режима можно оставить `Any`, но в production — только явные домены
- **Приоритет:** 🔴 Высокий

### 3. Rate Limiting
- **Проблема:** Нет защиты от brute-force атак на `/api/v1/auth/login`, flood WebSocket-подключений, злоупотребления API.
- **Решение:**
  - Добавить зависимость `tower-governor` или `axum-governor`
  - Middleware для HTTP: 100 req/min для обычных endpoints, 10 req/min для `/auth/login`
  - WebSocket: ограничить частоту новых подключений с одного IP
  - Секция в конфиге:
    ```toml
    [rate_limit]
    api_requests_per_minute = 100
    login_attempts_per_minute = 10
    ws_connections_per_minute = 20
    ```
- **Файлы:** новый `src/interfaces/http/middleware/rate_limit.rs`, `src/config.rs`
- **Приоритет:** 🔴 Высокий

---

## 🟠 Важные (надёжность / observability)

### 4. Обработка дублирующих WebSocket-подключений
- **Файл:** `src/interfaces/ws/ocpp_server.rs`, `src/application/charging/session/registry.rs`
- **Проблема:** Если станция переподключается, а старая сессия ещё висит — `SessionRegistry::register()` молча перезаписывает старую `Connection`. Старый sender канал остаётся "orphaned".
- **Решение:**
  - В `register()`: если сессия уже существует — сначала закрыть старый sender и опубликовать `ChargePointDisconnectedEvent`
  - Добавить debounce/backoff: если станция переподключается чаще чем раз в 5 секунд — отклонять
  - Логировать reconnection pattern для мониторинга

### 5. Метрики / Prometheus
- **Проблема:** Нет endpoint `/metrics` для мониторинга (Prometheus, Grafana).
- **Решение:**
  - Добавить `metrics` + `metrics-exporter-prometheus` crates
  - Трекать:
    - `ocpp_connected_stations` (gauge)
    - `ocpp_transactions_total` (counter, labels: status)
    - `ocpp_command_latency_seconds` (histogram)
    - `http_requests_total` (counter, labels: method, path, status)
    - `ws_messages_total` (counter, labels: direction, action)
  - Endpoint: `GET /metrics` (без auth)
- **Файлы:** новый `src/interfaces/http/modules/metrics/`, `Cargo.toml`
- **Приоритет:** 🟠 Важный

### 6. Request ID / Correlation ID
- **Проблема:** Нет сквозного ID запроса для трейсинга через логи.
- **Решение:**
  - Middleware: генерировать `X-Request-Id` UUID для каждого HTTP-запроса
  - Пробрасывать в `tracing::Span` для всех логов в рамках запроса
  - Для WebSocket: использовать `charge_point_id` + message `unique_id` как correlation
  - Зависимость: `tower-http::request-id`
- **Файлы:** `src/interfaces/http/middleware/`, `router.rs`
- **Приоритет:** 🟠 Важный

### 7. Восстановление соединения с БД
- **Проблема:** Если БД временно недоступна — все запросы падают без retry.
- **Решение:**
  - SeaORM уже поддерживает connection pool (`max_connections`, `min_connections`, `connect_timeout`)
  - Настроить в `config.rs`:
    ```toml
    [database]
    max_connections = 10
    min_connections = 2
    connect_timeout_seconds = 5
    idle_timeout_seconds = 300
    ```
  - Для критических операций (billing, stop_transaction) — добавить retry с backoff
- **Файлы:** `src/infrastructure/database/mod.rs`, `src/config.rs`
- **Приоритет:** 🟠 Важный

### 8. OCPP 2.0.1 CS→CP команды (OcppOutboundPort)
- **Файл:** `src/application/ports/outbound.rs`
- **Проблема:** Комментарий `"Phase 2: This trait will be fully implemented with version-specific adapters"`. CS→CP команды для V2.0.1 (`RequestStartTransaction`, `RequestStopTransaction`, `SetVariables`, `GetVariables`) используют V1.6 frame-формат через `CommandSender`.
- **Решение:**
  - Реализовать `OcppOutboundPort` для V2.0.1 с правильными типами сообщений из `rust_ocpp::v2_0_1`
  - Маршрутизировать команды через `Connection::ocpp_version` для выбора правильного сериализатора
  - Добавить V2.0.1-специфичные команды: `SetVariables`, `GetVariables`, `ClearChargingProfile`, `SetChargingProfile`
- **Приоритет:** 🟠 Важный (если планируется production-поддержка V2.0.1)

---

## 🟡 Средние (качество / DX)

### 9. Тесты
- **Текущее состояние:** ~10 unit-тестов (только `protocol_negotiation` и `ocpp_frames`)
- **Нужно покрыть:**
  - **Unit-тесты:**
    - `BillingService::calculate_transaction_billing` — разные тарифы, edge cases (0 energy, 0 duration)
    - `AppConfig::validate` — невалидные конфиги, пограничные значения
    - `ChargePointService` — register/update, start/stop transactions
    - `SessionRegistry` — register, unregister, concurrent access
    - `EventBus` — publish/subscribe, filtering, lag handling
    - `CommandSender` — timeout, cleanup, handle_response
    - Auth middleware — JWT expiry, invalid tokens, API key scopes
  - **Integration-тесты:**
    - Полный flow: WS connect → BootNotification → StartTransaction → MeterValues → StopTransaction → Billing
    - HTTP API: CRUD operations, auth flow, command sending
  - **Инфраструктура:**
    - Создать `tests/` директорию
    - Утилиты: `TestDb` (in-memory SQLite), mock `SessionRegistry`, test fixtures
- **Файлы:** `tests/`, inline `#[cfg(test)] mod tests` в сервисах
- **Приоритет:** 🟡 Средний

### 10. Docker / Deployment
- **Проблема:** Нет Dockerfile, docker-compose, CI/CD конфигов.
- **Решение:**
  - `Dockerfile` — multi-stage build (builder + runtime)
  - `docker-compose.yml` — сервис + PostgreSQL + (опционально) Prometheus + Grafana
  - `.github/workflows/ci.yml` — cargo check, cargo test, cargo clippy, cargo fmt
  - `.github/workflows/release.yml` — build бинарников для Linux/macOS/Windows
- **Файлы:** корень проекта
- **Приоритет:** 🟡 Средний

### 11. Structured Logging (JSON формат)
- **Проблема:** Логи в текстовом формате — неудобно для агрегации (ELK, Loki).
- **Решение:**
  - Добавить `tracing-subscriber` с `json` layer
  - Конфиг:
    ```toml
    [logging]
    level = "info"
    format = "json"  # или "text"
    ```
  - В production — `json`, в dev — `text` (human-readable)
- **Файлы:** `src/main.rs` (инициализация tracing), `src/config.rs`
- **Приоритет:** 🟡 Средний

### 12. Environment Variables для секретов
- **Проблема:** Секреты (JWT secret, DB password, admin password) хранятся только в TOML-файле. Нет поддержки env vars.
- **Решение:**
  - Добавить `config` crate или вручную через `std::env::var`:
    ```
    OCPP_JWT_SECRET=... → переопределяет [security].jwt_secret
    OCPP_DB_PASSWORD=... → переопределяет [database.postgres].password
    OCPP_ADMIN_PASSWORD=... → переопределяет [admin].password
    ```
  - Env vars имеют приоритет над TOML
- **Файлы:** `src/config.rs`
- **Приоритет:** 🟡 Средний

### 13. Валидация входных данных (request body)
- **Проблема:** Нет единого слоя валидации. Проверки разбросаны по хэндлерам ad-hoc.
- **Решение:**
  - Добавить `validator` crate
  - Derive `#[derive(Validate)]` на все DTO:
    ```rust
    #[derive(Validate)]
    struct RemoteStartRequest {
        #[validate(length(min = 1, max = 20))]
        id_tag: String,
        #[validate(range(min = 1, max = 10))]
        connector_id: Option<u32>,
    }
    ```
  - Axum extractor: `Json<Valid<T>>` → автоматический 400 при невалидных данных
- **Файлы:** `src/interfaces/http/common/`, DTO модули
- **Приоритет:** 🟡 Средний

---

## 🟢 Низкие (nice-to-have)

### 14. gRPC интерфейс
- **Файл:** `src/interfaces/grpc/mod.rs` — пустой placeholder
- **Описание:** Для межсервисного взаимодействия (микросервисы, mobile backend, внешние интеграции).
- **Решение:** `tonic` + `.proto` файлы для основных операций (RemoteStart/Stop, GetStatus, Transactions)
- **Приоритет:** 🟢 Низкий — REST API покрывает текущие потребности

### 15. WebSocket Ping/Pong keepalive для OCPP
- **Файл:** `src/interfaces/ws/ocpp_server.rs`
- **Описание:** Сервер не отправляет периодические ping к станциям. Полагается только на OCPP Heartbeat.
- **Решение:** Добавить `tokio::interval` для отправки WS Ping каждые 30с. Если Pong не получен за 10с — считать соединение мёртвым и закрывать.
- **Приоритет:** 🟢 Низкий (Heartbeat частично покрывает)

### 16. Audit Log
- **Описание:** Логировать в БД все значимые действия: кто отправил RemoteStart, кто заблокировал IdTag, кто изменил тариф.
- **Решение:**
  - Новая таблица `audit_logs` (timestamp, user_id, action, entity, entity_id, details)
  - Middleware или event listener для записи
- **Приоритет:** 🟢 Низкий

### 17. OCPP 2.1 Support
- **Файл:** `src/main.rs` — комментарий `// Future: protocol_adapters.register(OcppVersion::V21, v21_factory)`
- **Описание:** `OcppVersion::V21` уже есть в enum, но adapter не реализован.
- **Приоритет:** 🟢 Низкий — V21 стандарт ещё не широко поддержан станциями

### 18. Limit Body Size
- **Описание:** Нет ограничения на размер HTTP request body. Потенциальный DDoS-вектор.
- **Решение:** `tower_http::limit::RequestBodyLimitLayer::new(1_048_576)` (1 MB) в `router.rs`
- **Приоритет:** 🟢 Низкий

---

## 📝 Технические долги (известные)

| Место | Описание |
|-------|----------|
| `src/application/ports/outbound.rs` | `OcppOutboundPort` — "Phase 2" stub, не реализован |
| `src/interfaces/grpc/mod.rs` | Пустой placeholder |
| `handle_stop_transaction.rs` / v16 | Нет проверки `id_tag` авторизации при StopTransaction |
| `ocpp_server.rs` L67 | Fallback на последнюю версию при неизвестном subprotocol — может подключить станцию на неправильном протоколе |
| `SessionRegistry::register()` | Молча перезаписывает существующую сессию при reconnect |
| `remote_stop` handler | Proactive stop в HTTP handler дублирует логику StopTransaction OCPP handler — DRY нарушение |
| `force_stop_transaction` | Использует `meter_start` как `meter_stop` — неточный расчёт energy |

---

## ✅ Полностью реализовано (не требует работы)

- [x] Clean Architecture / DDD агрегаты
- [x] TOML конфигурация с валидацией
- [x] SeaORM + миграции (SQLite / PostgreSQL)
- [x] Graceful shutdown (SIGTERM/SIGINT + timeout)
- [x] OCPP 1.6 полный (CP→CS + CS→CP)
- [x] OCPP 2.0.1 CP→CS хэндлеры
- [x] Session Registry (DashMap)
- [x] REST API (40+ endpoints) + Swagger UI
- [x] JWT + API Key аутентификация
- [x] Event Bus + WebSocket Notifications (12 event types)
- [x] Billing (energy + time + session fee)
- [x] Heartbeat Monitor с auto-status transitions
- [x] Health Check с DB ping + uptime
- [x] TransactionBilledEvent
- [x] Авто-биллинг при RemoteStop / ForceStop
- [x] Default admin creation
