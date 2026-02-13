# TODO — Texnouz OCPP Central System

> Дорожная карта до production-ready CSMS.
> Обновлено: 2026-02-13

---

## 📊 Текущий статус OCPP покрытия

```
OCPP 1.6 CP→CS:   ██████████████████████████████  100%  (11/11)
OCPP 1.6 CS→CP:   ███████████░░░░░░░░░░░░░░░░░░░   55%  (11/20)
OCPP 2.0.1 CP→CS: ███████████████░░░░░░░░░░░░░░░   50%  (9/18)
OCPP 2.0.1 CS→CP: █████████████░░░░░░░░░░░░░░░░░   46%  (13/28)
Security:          ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░   10%
Business Logic:    ████████████████░░░░░░░░░░░░░░░   55%
Infrastructure:    █████████████████████████████░░   95%
```

---

## Phase 1 — 🔴 Security & Core Commands

### 1. Аутентификация WebSocket-подключений зарядных станций
- **Файл:** `src/interfaces/ws/ocpp_server.rs` → `handle_connection()`
- **Проблема:** Любое устройство может подключиться по `ws://<host>:9000/<charge_point_id>` без проверки.
- **Решение:**
  - **OCPP 1.6 Security Profile 1:** Basic Auth — `Authorization` header при WS upgrade. Пароль хранится в таблице `charge_points`.
  - **OCPP 2.0.1 Security Profile 1-3:** Basic Auth → TLS client certificates → Mutual TLS.
  - **Whitelist:** Отклонять неизвестные `charge_point_id` (не зарегистрированные в БД).
  - Конфиг:
    ```toml
    [security]
    ws_auth_mode = "basic"  # "none" | "basic" | "token" | "certificate"
    reject_unknown_charge_points = true
    ```
- **Файлы:** `ocpp_server.rs`, `config.rs`, миграция `charge_points` (добавить `password_hash`)
- **Приоритет:** 🔴 Критический — без этого любой может имитировать станцию

### 2. SendLocalList (v1.6 + v2.0.1)
- **Проблема:** Нет офлайн-авторизации. Если станция теряет связь с CSMS, она не может авторизовать RFID-карты.
- **Решение:**
  - CS→CP: `SendLocalList` — отправка списка разрешённых IdTag на станцию
  - Хэндлер v1.6: `src/application/handlers/ocpp_v16/send_local_list.rs`
  - Хэндлер v2.0.1: `src/application/handlers/ocpp_v201/send_local_list.rs`
  - HTTP endpoint: `POST /api/v1/charge-points/{id}/local-list` (отправить актуальный список)
  - Автоматическая отправка при подключении станции (если `auto_sync_local_list = true`)
- **Файлы:** `commands/`, `OcppOutboundPort`, HTTP handler
- **Приоритет:** 🔴 Критический для офлайн-сценариев

### 3. SetChargingProfile / ClearChargingProfile / GetCompositeSchedule (v1.6)
- **Проблема:** Smart Charging реализован только для v2.0.1, но не для v1.6 — а это самая распространённая версия.
- **Решение:**
  - `src/application/commands/v16/set_charging_profile.rs`
  - `src/application/commands/v16/clear_charging_profile.rs`
  - `src/application/commands/v16/get_composite_schedule.rs`
  - HTTP endpoints для v1.6 станций
  - DB-таблица `charging_profiles` для хранения активных профилей
- **Файлы:** `commands/v16/`, миграция, HTTP handlers
- **Приоритет:** 🔴 Без этого нельзя ограничивать мощность зарядки на v1.6 станциях

### 4. ReserveNow / CancelReservation (v1.6 + v2.0.1)
- **Проблема:** Пользователь не может забронировать коннектор через приложение — стандартная фича публичного CSMS.
- **Решение:**
  - DB: таблица `reservations` (id, charge_point_id, connector_id, id_tag, expiry_date, status)
  - Домен: `Reservation` модель, `ReservationRepository`
  - CS→CP: `ReserveNow`, `CancelReservation` для v1.6 и v2.0.1
  - Интеграция с `Authorize` хэндлером — проверка: есть ли бронь на этот коннектор для другого пользователя?
  - Автоматическая отмена по expiry (фоновая задача)
  - HTTP endpoints: `POST /reservations`, `DELETE /reservations/{id}`, `GET /reservations`
- **Файлы:** `domain/reservation/`, `commands/`, миграция, HTTP module
- **Приоритет:** 🔴 Обязательная фича для коммерческой CSMS

### 5. Firmware Management (v1.6 + v2.0.1)
- **Проблема:** Нет удалённого обновления прошивки и получения диагностики — требуется физический выезд на каждую станцию.
- **Решение:**
  - **v1.6:** `UpdateFirmware` + `GetDiagnostics` (CS→CP)
  - **v2.0.1:** `UpdateFirmware` + `GetLog` (CS→CP)
  - CP→CS: `FirmwareStatusNotification`, `DiagnosticsStatusNotification` — уже обработаны
  - DB: таблица `firmware_tasks` (id, charge_point_id, type, url, status, requested_at, completed_at)
  - HTTP endpoints: `POST /charge-points/{id}/firmware/update`, `POST /charge-points/{id}/diagnostics`
- **Файлы:** `commands/v16/`, `commands/v201/`, миграция, HTTP handlers
- **Приоритет:** 🟠 Важный для операционного обслуживания

---

## Phase 2 — 🟠 Device Management & Monitoring

### 6. GetBaseReport + NotifyReport (v2.0.1)
- **Проблема:** CSMS не может запросить полный отчёт о переменных/компонентах станции. Без этого нет device management для v2.0.1.
- **Решение:**
  - CS→CP: `GetBaseReport` — запрос отчёта (ConfigurationInventory, FullInventory)
  - CP→CS handler: `NotifyReport` — приём отчёта (может прийти несколько частей, `tbc=true/false`)
  - DB: таблица `device_reports` или кэш в памяти
  - HTTP endpoint: `POST /charge-points/{id}/report`
- **Файлы:** `commands/v201/get_base_report.rs`, `handlers/ocpp_v201/handle_notify_report.rs`
- **Приоритет:** 🟠 Важный

### 7. NotifyEvent + мониторинг переменных (v2.0.1)
- **Проблема:** Станция может сообщать об аномалиях (перегрев, ошибки заземления, превышение тока). CSMS их игнорирует.
- **Решение:**
  - CP→CS handler: `NotifyEvent` — обработка событий мониторинга
  - CS→CP: `SetVariableMonitoring` — настройка порогов (alert при temperature > 60°C)
  - CS→CP: `SetMonitoringBase` — включение мониторинга
  - CP→CS handler: `NotifyMonitoringReport` — отчёт о настроенных мониторах
  - EventBus: новый `DeviceAlertEvent` для уведомлений
- **Файлы:** `handlers/ocpp_v201/`, `commands/v201/`, `domain/events/types.rs`
- **Приоритет:** 🟠 Важный для safety

### 8. Charging Profiles DB management
- **Проблема:** ChargingProfiles отправляются ad-hoc через API — не сохраняются, не привязаны к станциям. Нет истории и управления.
- **Решение:**
  - DB: таблица `charging_profiles` (id, charge_point_id, evse_id, stack_level, purpose, kind, schedule_json, is_active, created_at)
  - Домен: `ChargingProfile` модель, `ChargingProfileRepository`
  - При `SetChargingProfile` — сохранять в БД, при `ClearChargingProfile` — помечать inactive
  - CS→CP v2.0.1: `GetChargingProfiles` — запрос активных профилей со станции
  - CP→CS v2.0.1: `ReportChargingProfiles` — ответ со списком
  - HTTP: `GET /charge-points/{id}/charging-profiles`
- **Файлы:** `domain/charging_profile/`, миграция, `commands/v201/`, HTTP module
- **Приоритет:** 🟠 Важный для Smart Charging

### 9. GetTransactionStatus (v2.0.1)
- **Проблема:** Нет способа проверить текущее состояние транзакции на станции (например после reconnect).
- **Решение:**
  - CS→CP: `GetTransactionStatus` — запрос статуса конкретной транзакции
  - HTTP endpoint: `GET /charge-points/{id}/transactions/{txId}/status`
- **Файлы:** `commands/v201/get_transaction_status.rs`
- **Приоритет:** 🟡 Средний

---

## Phase 3 — 🟡 Business Logic & Integrations

### 10. IdTag авторизация при StopTransaction
- **Файл:** `handle_stop_transaction.rs` (v1.6)
- **Проблема:** Кто угодно может остановить чужую зарядку — нет проверки `id_tag` при StopTransaction.
- **Решение:**
  - Проверить: `stop_id_tag == start_id_tag` или `stop_id_tag.parent == start_id_tag`
  - Если нет — отклонить (или разрешить, но логировать как аномалию)
- **Файлы:** `handlers/ocpp_v16/handle_stop_transaction.rs`
- **Приоритет:** 🟡 Средний

### 11. Webhook/Callback система
- **Проблема:** Внешние системы (мобильное приложение, CRM, биллинг) не могут получать realtime уведомления.
- **Решение:**
  - DB: таблица `webhooks` (id, url, events[], secret, is_active, created_at)
  - HTTP: CRUD endpoints `/api/v1/webhooks`
  - EventBus subscriber → HTTP POST к зарегистрированным webhook URL
  - HMAC-SHA256 подпись тела (`X-Webhook-Signature`)
  - Retry с exponential backoff (3 попытки)
  - Поддерживаемые события: `transaction.started`, `transaction.stopped`, `transaction.billed`, `charge_point.connected`, `charge_point.disconnected`, `charge_point.status_changed`
- **Файлы:** `domain/webhook/`, миграция, `application/services/webhook.rs`, HTTP module
- **Приоритет:** 🟡 Средний

### 12. Dashboard / Analytics API
- **Проблема:** Нет агрегированных данных для отображения на dashboard frontend.
- **Решение:**
  - `GET /api/v1/analytics/summary` — общая сводка (stations online/offline, active transactions, revenue today/month)
  - `GET /api/v1/analytics/revenue?period=day|week|month` — выручка по периодам
  - `GET /api/v1/analytics/energy?period=day|week|month` — потреблённая энергия
  - `GET /api/v1/analytics/peak-hours` — часы пиковой нагрузки
  - `GET /api/v1/analytics/station-uptime` — uptime по станциям
- **Файлы:** `src/interfaces/http/modules/analytics/`
- **Приоритет:** 🟡 Средний

### 13. Audit Log
- **Проблема:** Нет истории кто, когда и что сделал.
- **Решение:**
  - DB: таблица `audit_logs` (id, timestamp, user_id, action, entity_type, entity_id, details_json, ip_address)
  - Middleware: автоматическая запись для мутирующих HTTP-запросов (POST, PUT, DELETE)
  - HTTP: `GET /api/v1/audit-logs?entity=charge_point&entity_id=CP001`
- **Файлы:** `domain/audit/`, миграция, middleware, HTTP module
- **Приоритет:** 🟡 Средний

### 14. Notification система (Email/SMS/Push)
- **Проблема:** Оператор не узнает о проблемах, пока сам не посмотрит dashboard.
- **Решение:**
  - DB: таблица `notification_rules` (id, event_type, channel, recipient, is_active)
  - Каналы: Email (SMTP/SendGrid), Telegram Bot, SMS (опционально)
  - Триггеры: станция offline > 5 мин, транзакция failed, ошибка коннектора
  - Конфиг:
    ```toml
    [notifications]
    enabled = true
    telegram_bot_token = "..."
    smtp_host = "..."
    ```
- **Файлы:** `application/services/notification.rs`, `config.rs`
- **Приоритет:** 🟡 Средний

---

## Phase 4 — 🟢 Advanced Features

### 15. Multi-tenancy
- **Проблема:** Один CSMS — один оператор. Для SaaS-модели нужна изоляция данных между организациями.
- **Решение:**
  - DB: таблица `organizations` (id, name, slug, settings_json)
  - Добавить `organization_id` во все основные таблицы (charge_points, transactions, tariffs, users, id_tags)
  - Middleware: определение организации из JWT / subdomain / API key
  - Фильтрация всех запросов по `organization_id`
- **Файлы:** миграции, middleware, все repositories
- **Приоритет:** 🟢 Для SaaS

### 16. OCPI 2.2.1 (роуминг между операторами)
- **Проблема:** Зарядка возможна только картой своего оператора. Для публичных сетей нужен роуминг.
- **Решение:**
  - OCPI 2.2.1 — REST-based протокол для обмена данными между CPO (оператор станций) и eMSP (провайдер карт)
  - Модули: Locations, Sessions, CDRs, Tariffs, Tokens, Commands
  - Новый HTTP router: `/ocpi/2.2.1/...`
- **Файлы:** `src/interfaces/ocpi/` (новый модуль)
- **Приоритет:** 🟢 Для публичных сетей

### 17. Payment Gateway интеграция
- **Проблема:** Биллинг рассчитывает стоимость, но нет реального списания денег.
- **Решение:**
  - Абстракция: `PaymentGateway` trait (`authorize`, `capture`, `refund`)
  - Реализации: Click/Payme (Узбекистан), Stripe (международный)
  - Поток: `TransactionStarted` → `authorize(limit)` → `TransactionBilled` → `capture(total_cost)`
- **Файлы:** `infrastructure/payment/`, `application/services/payment.rs`
- **Приоритет:** 🟢 Для коммерческой эксплуатации

### 18. OCPP 2.0.1 Security Profiles (Certificates)
- **Проблема:** Нет TLS certificate management для v2.0.1 Security Profile 2/3.
- **Решение:**
  - CP→CS: `SignCertificate` — станция запрашивает подпись CSR
  - CS→CP: `CertificateSigned` — отправка подписанного сертификата
  - CS→CP: `InstallCertificate`, `DeleteCertificate`, `GetInstalledCertificateIds`
  - Интеграция с CA (Let's Encrypt / внутренний CA)
- **Файлы:** `handlers/ocpp_v201/`, `commands/v201/`, `infrastructure/crypto/`
- **Приоритет:** 🟢 Для enterprise

### 19. WebSocket Ping/Pong keepalive
- **Файл:** `src/interfaces/ws/ocpp_server.rs`
- **Описание:** Сервер не шлёт WS Ping. Полагается только на OCPP Heartbeat.
- **Решение:** `tokio::interval` → WS Ping каждые 30с. Нет Pong за 10с → закрыть соединение.
- **Приоритет:** 🟢 Heartbeat частично покрывает

### 20. OCPP 2.1 Support
- `OcppVersion::V21` есть в enum, adapter не реализован.
- **Приоритет:** 🟢 Стандарт ещё не широко поддержан станциями

### 21. Limit HTTP Body Size
- Нет ограничения на размер request body. Потенциальный DDoS-вектор.
- **Решение:** `RequestBodyLimitLayer::new(1_048_576)` (1 MB) в `router.rs`
- **Приоритет:** 🟢

### 22. gRPC интерфейс
- `src/interfaces/grpc/mod.rs` — пустой placeholder.
- `tonic` + `.proto` для межсервисного взаимодействия.
- **Приоритет:** 🟢 REST API покрывает текущие потребности

---

## 📝 Технические долги

| Место | Описание |
|-------|----------|
| `handle_stop_transaction.rs` (v1.6) | Нет проверки `id_tag` авторизации при StopTransaction |
| `ocpp_server.rs` L67 | Fallback на последнюю версию при неизвестном subprotocol |
| `remote_stop` handler | Proactive stop дублирует логику StopTransaction OCPP handler (DRY) |
| `force_stop_transaction` | Использует `meter_start` как `meter_stop` — неточный расчёт energy |
| `src/interfaces/grpc/mod.rs` | Пустой placeholder |
| Smart Charging v1.6 | Реализовано только для v2.0.1, нет для v1.6 |
| `CorsLayer` | Настройки из `config.rs` применяются, но `*` всё ещё допустим без предупреждения |

---

## ✅ Полностью реализовано

- [x] Clean Architecture / DDD агрегаты
- [x] TOML конфигурация с валидацией
- [x] SeaORM + миграции (SQLite / PostgreSQL)
- [x] Graceful shutdown (SIGTERM/SIGINT + timeout)
- [x] OCPP 1.6 CP→CS (11/11 сообщений)
- [x] OCPP 1.6 CS→CP базовые (11/20 команд)
- [x] OCPP 2.0.1 CP→CS хэндлеры (9/18)
- [x] OCPP 2.0.1 CS→CP (13/28 команд)
- [x] Session Registry (DashMap + eviction + debounce)
- [x] REST API (40+ endpoints) + Swagger UI
- [x] JWT + API Key аутентификация
- [x] Event Bus + WebSocket Notifications (12 event types)
- [x] Billing (energy + time + session fee + авто-биллинг)
- [x] Heartbeat Monitor с auto-status transitions
- [x] Health Check с DB ping + uptime
- [x] Default admin creation
- [x] OcppOutboundPort trait (полный)
- [x] Request ID / Correlation ID (HTTP + WS)
- [x] DB Connection Pool + Retry с backoff
- [x] Prometheus метрики (8 типов)
- [x] CORS конфигурация
- [x] Rate Limiting (HTTP + WS)
- [x] Structured Logging (JSON/text)
- [x] Environment Variables (10 overrides)
- [x] Input Validation (validator + ValidatedJson)
- [x] Docker (Dockerfile + docker-compose + Prometheus + Grafana)
- [x] CI/CD (GitHub Actions: fmt + clippy + test + docker)
- [x] 88 unit-тестов
