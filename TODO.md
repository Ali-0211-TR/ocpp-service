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

### ~~8. OCPP 2.0.1 CS→CP команды (OcppOutboundPort)~~ ✅
- **Реализовано:** `OcppOutboundPort` trait полностью реализован. `CommandDispatcher` имплементирует trait. V2.0.1 команды: `ClearChargingProfile`, `SetChargingProfile` добавлены. HTTP endpoints: `/variables/get`, `/variables/set`, `/charging-profile/clear`, `/charging-profile/set`.
- **Приоритет:** 🟠 Важный (если планируется production-поддержка V2.0.1)

---

## 🟡 Средние (качество / DX)

### ~~9. Тесты~~ ✅
- **Реализовано:** 88 unit-тестов (было 10 → стало 88). Покрыты: `Tariff::calculate_cost/cost_breakdown` (все типы тарифов, min/max fee, is_valid), `Transaction` (create/stop/energy/limits), `AppConfig::validate` (19 тестов: порты, JWT, пароли, уровни логов, формат, env overrides, save/reload), `EventBus` (pub/sub, subscriber count, drop), `SessionRegistry` (register/evict/unregister/debounce/broadcast/touch), `Connection` (send/stale/touch), `ValidatedJson` extractor (200/400/422).
- **Приоритет:** 🟡 Средний

### ~~10. Docker / Deployment~~ ✅
- **Реализовано:** `Dockerfile` (multi-stage: rust:1.82-bookworm builder → debian:bookworm-slim runtime, non-root user, health check). `docker-compose.yml` (OCPP service + Prometheus + Grafana, volumes, environment overrides). `.dockerignore`. `deploy/prometheus.yml`. `.github/workflows/ci.yml` (fmt + clippy + check + test + docker build).
- **Приоритет:** 🟡 Средний

### ~~11. Structured Logging (JSON формат)~~ ✅
- **Реализовано:** `LoggingConfig.format` поле (`"text"` | `"json"`). `tracing_subscriber::registry()` с условным JSON или text layer. Конфигурация через TOML `[logging].format` или env `OCPP_LOG_FORMAT`.
- **Приоритет:** 🟡 Средний

### ~~12. Environment Variables для секретов~~ ✅
- **Реализовано:** `AppConfig::apply_env_overrides()` поддерживает 10 env vars: `OCPP_JWT_SECRET`, `OCPP_DB_PASSWORD`, `OCPP_ADMIN_PASSWORD`, `OCPP_ADMIN_USERNAME`, `OCPP_ADMIN_EMAIL`, `OCPP_LOG_LEVEL`, `OCPP_LOG_FORMAT`, `OCPP_API_PORT`, `OCPP_WS_PORT`. Env vars имеют приоритет над TOML.
- **Приоритет:** 🟡 Средний

### ~~13. Валидация входных данных (request body)~~ ✅
- **Реализовано:** `validator 0.18` с `derive`. `ValidatedJson<T>` custom Axum extractor (422 с field-level ошибками). `#[derive(Validate)]` на все request DTO: auth (Login, Register, ChangePassword), users (Create, Update), id_tags (Create, Update), tariffs (Create, Update, CostPreview), commands (RemoteStart, RemoteStop, Reset, ChangeAvailability, TriggerMessage, DataTransfer, GetVariables, SetVariables, ClearChargingProfile, SetChargingProfile), api_keys (Create), charge_points (CreateConnector).
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
- [x] OCPP 2.0.1 CS→CP полный (OcppOutboundPort + ClearChargingProfile + SetChargingProfile)
- [x] Request ID / Correlation ID (HTTP + WS)
- [x] DB Connection Pool + Retry с backoff
- [x] Prometheus метрики (8 типов)
- [x] Обработка дублирующих WS-подключений (eviction + debounce)
- [x] CORS конфигурация
- [x] Rate Limiting (HTTP + WS)
- [x] Structured Logging (JSON/text format)
- [x] Environment Variables (10 env overrides)
- [x] Input Validation (validator + ValidatedJson extractor)
- [x] Docker / Deployment (Dockerfile + docker-compose + CI/CD)
- [x] 88 unit-тестов (tariff, transaction, config, event_bus, session_registry, connection, validated_json)
