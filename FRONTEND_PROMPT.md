# Промпт для AI-агента: React Admin Panel для OCPP Central System

## Роль

Ты — senior full-stack разработчик. Создай production-ready React SPA админ-панель для управления OCPP 1.6 зарядными станциями.

---

## Стек технологий

| Категория | Технология |
|-----------|-----------|
| Фреймворк | React 18 + TypeScript 5 |
| Сборщик | Vite 6 |
| Роутинг | React Router v7 |
| Стейт | Zustand 5 + React Query (TanStack Query v5) |
| UI | Tailwind CSS 4 + shadcn/ui |
| Таблицы | TanStack Table v8 |
| Графики | Recharts 2 |
| Формы | React Hook Form + Zod |
| HTTP | Axios |
| WebSocket | native WebSocket (reconnecting) |
| Иконки | Lucide React |
| Тема | Тёмная (Catppuccin Mocha / custom dark) |
| Язык интерфейса | Русский |

---

## Backend API

**Base URL:** `http://localhost:8080`
**Swagger UI:** `http://localhost:8080/docs`
**OpenAPI JSON:** `http://localhost:8080/api-doc/openapi.json`

### Аутентификация

Два метода (оба равнозначны, middleware проверяет по порядку):

| Метод | Header | Как получить |
|-------|--------|-------------|
| JWT Bearer | `Authorization: Bearer <token>` | `POST /api/v1/auth/login` → `{ token, expires_in }` |
| API Key | `X-API-Key: <key>` или `Authorization: <key>` | `POST /api/v1/api-keys` → ключ показывается **один раз** |

**Роли:** `admin`, `operator`, `viewer`

### Универсальный формат ответов

```typescript
// Все эндпоинты оборачивают ответ в:
interface ApiResponse<T> {
  success: boolean;
  data?: T;       // null при ошибке
  error?: string; // null при успехе
}

interface PaginatedResponse<T> {
  items: T[];
  total: number;     // общее количество
  page: number;      // текущая страница (1-based)
  limit: number;     // размер страницы
  total_pages: number;
}
```

---

## Полный список API эндпоинтов (38 REST + 1 WS)

### 🟢 Health (публичный)

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/health` | Проверка состояния сервиса |

### 🔑 Auth

| Метод | Путь | Auth | Описание | Body |
|-------|------|------|----------|------|
| POST | `/api/v1/auth/login` | ❌ | Логин | `{ username, password }` → `{ token, expires_in, user }` |
| POST | `/api/v1/auth/register` | ❌ | Регистрация | `{ username, email, password, role? }` |
| GET | `/api/v1/auth/me` | 🔒 JWT | Текущий пользователь | — |
| PUT | `/api/v1/auth/change-password` | 🔒 JWT | Смена пароля | `{ current_password, new_password }` |

### 🗝️ API Keys (только JWT)

| Метод | Путь | Описание | Body |
|-------|------|----------|------|
| POST | `/api/v1/api-keys` | Создать ключ | `{ name, scopes?, expires_in_days? }` → **ключ показывается один раз!** |
| GET | `/api/v1/api-keys` | Список ключей | — |
| DELETE | `/api/v1/api-keys/{id}` | Отозвать ключ | — |

### 🏷️ IdTags (RFID-карты)

| Метод | Путь | Описание | Body / Query |
|-------|------|----------|-------------|
| GET | `/api/v1/id-tags` | Список (фильтры, пагинация) | `?status=&is_active=&user_id=&page=&page_size=` |
| GET | `/api/v1/id-tags/{id_tag}` | Получить по значению | — |
| POST | `/api/v1/id-tags` | Создать | `{ id_tag, parent_id_tag?, status, user_id?, name?, expiry_date?, max_active_transactions? }` |
| PUT | `/api/v1/id-tags/{id_tag}` | Обновить (partial) | Все поля опциональны |
| DELETE | `/api/v1/id-tags/{id_tag}` | Удалить | — |
| POST | `/api/v1/id-tags/{id_tag}/block` | Заблокировать | — |
| POST | `/api/v1/id-tags/{id_tag}/unblock` | Разблокировать | — |

### 💰 Tariffs (тарифы)

| Метод | Путь | Описание | Body |
|-------|------|----------|------|
| GET | `/api/v1/tariffs` | Список всех | — |
| GET | `/api/v1/tariffs/default` | Тариф по умолчанию | — |
| GET | `/api/v1/tariffs/{id}` | По ID | — |
| POST | `/api/v1/tariffs` | Создать | `{ name, description?, tariff_type, price_per_kwh, price_per_minute, session_fee, currency, min_fee?, max_fee?, is_active?, is_default?, valid_from?, valid_until? }` |
| PUT | `/api/v1/tariffs/{id}` | Обновить (partial) | Все поля опциональны |
| DELETE | `/api/v1/tariffs/{id}` | Удалить | — |
| POST | `/api/v1/tariffs/preview-cost` | Предварительный расчёт | `{ tariff_id?, energy_wh, duration_seconds }` → `{ energy_cost, time_cost, session_fee, subtotal, total, currency, formatted_total }` |

### ⚡ Charge Points (станции)

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/api/v1/charge-points` | Список всех станций |
| GET | `/api/v1/charge-points/stats` | Статистика: total, online, offline, charging |
| GET | `/api/v1/charge-points/online` | Список ID онлайн-станций |
| GET | `/api/v1/charge-points/{id}` | Детали станции |
| DELETE | `/api/v1/charge-points/{id}` | Удалить станцию |

### 🔧 OCPP Commands (станция должна быть online)

| Метод | Путь | Описание | Body |
|-------|------|----------|------|
| POST | `.../remote-start` | Запуск зарядки | `{ id_tag, connector_id? }` |
| POST | `.../remote-stop` | Остановка зарядки | `{ transaction_id }` |
| POST | `.../reset` | Перезагрузка | `{ type: "Soft" \| "Hard" }` |
| POST | `.../unlock-connector` | Разблокировка разъёма | `{ connector_id }` |
| POST | `.../change-availability` | Изменить доступность | `{ connector_id, type: "Operative" \| "Inoperative" }` |
| POST | `.../trigger-message` | Запросить сообщение | `{ message, connector_id? }` |
| **GET** | **`.../configuration`** | **Получить ВСЮ конфигурацию станции** | `?keys=` (опционально, через запятую) |
| **PUT** | **`.../configuration`** | **Изменить конфигурацию** | `{ key, value }` |
| **GET** | **`.../local-list-version`** | **Версия списка авторизации** | — → `{ list_version }` |
| **POST** | **`.../clear-cache`** | **Очистить кэш авторизации** | — |
| **POST** | **`.../data-transfer`** | **Произвольный обмен данными** | `{ vendor_id, message_id?, data? }` → `{ status, data? }` |

> Все пути выше начинаются с `/api/v1/charge-points/{charge_point_id}/`

**⚠️ Обработка ошибок команд:**
- **200** → станция ответила, результат в `CommandResponse.status` (`Accepted`, `Rejected`, `RebootRequired`, `NotSupported`)
- **404** → станция не подключена (offline)
- **500** → станция ответила ошибкой (`CallError`) или таймаут. Поле `error` содержит описание (напр. `"NotImplemented"`, `"InternalError"`, `"timeout"`). Это частый случай — некоторые станции не поддерживают отдельные команды (GetConfiguration, ClearCache, DataTransfer и др.)

Фронтенд должен показывать 500-ошибки команд как **«Станция не поддерживает эту команду»** (красный toast), а не как системную ошибку.

**Ответ команд:**
```typescript
interface CommandResponse {
  status: string;   // "Accepted" | "Rejected" | "RebootRequired" | "NotSupported" | ...
  message?: string; // Описание результата
}

interface ConfigurationResponse {
  configuration: ConfigValue[];
  unknown_keys: string[];
}

interface ConfigValue {
  key: string;
  value?: string;
  readonly: boolean;
}

interface LocalListVersionResponse {
  list_version: number; // -1 = не поддерживается, 0 = пуст, >0 = версия
}

interface DataTransferResponse {
  status: string; // "Accepted" | "Rejected" | "UnknownMessageId" | "UnknownVendorId"
  data?: string;
}
```

### 📊 Transactions

| Метод | Путь | Описание | Query |
|-------|------|----------|-------|
| GET | `/api/v1/transactions` | Все транзакции (пагинация) | `?page=&limit=` |
| GET | `/api/v1/transactions/{id}` | Одна транзакция | — |
| **POST** | **`/api/v1/transactions/{id}/force-stop`** | **Принудительная остановка зависшей транзакции** | — |
| GET | `.../charge-points/{id}/transactions` | Транзакции станции | `?status=&from_date=&to_date=&page=&limit=` |
| GET | `.../charge-points/{id}/transactions/active` | Активные транзакции | — |
| GET | `.../charge-points/{id}/transactions/stats` | Статистика | — |

```typescript
interface TransactionDto {
  id: number;
  charge_point_id: string;
  connector_id: number;
  id_tag: string;
  meter_start: number;    // Wh
  meter_stop?: number;    // Wh
  energy_consumed_wh?: number;
  status: "Active" | "Completed" | "Failed";
  started_at: string;     // ISO 8601
  stopped_at?: string;
  stop_reason?: string;   // "Remote" | "EVDisconnected" | "ForceStop" | ...
}

interface TransactionStats {
  total: number;
  active: number;
  completed: number;
  total_energy_kwh: number;
}
```

### 📡 Monitoring

| Метод | Путь | Описание |
|-------|------|----------|
| GET | `/api/v1/monitoring/heartbeats` | Heartbeat-статусы всех станций |
| GET | `/api/v1/monitoring/stats` | Статистика подключений |
| GET | `/api/v1/monitoring/online` | Онлайн-станции |

### 🔔 WebSocket Notifications

**URL:** `ws://localhost:8080/api/v1/notifications/ws`
**Query-фильтры:** `?charge_point_id=CP001&events=transaction_started,connector_status_changed`

При подключении сервер шлёт: `{"type":"connected","message":"Subscribed to OCPP events"}`

```typescript
interface WebSocketEvent {
  id: string;           // UUID
  timestamp: string;    // ISO 8601
  type: EventType;
  data: EventData;      // зависит от type
}

type EventType =
  | "charge_point_connected"
  | "charge_point_disconnected"
  | "charge_point_status_changed"
  | "connector_status_changed"
  | "transaction_started"
  | "transaction_stopped"
  | "meter_values_received"
  | "heartbeat_received"
  | "authorization_result"
  | "boot_notification"
  | "error";

// Ключевые data-типы:
interface TransactionStartedEvent {
  charge_point_id: string;
  connector_id: number;
  transaction_id: number;
  id_tag: string;
  meter_start: number;
  timestamp: string;
}

interface TransactionStoppedEvent {
  charge_point_id: string;
  transaction_id: number;
  id_tag?: string;
  meter_stop: number;
  energy_consumed_kwh: number;
  total_cost: number;
  currency: string;
  reason?: string;
  timestamp: string;
}

interface ConnectorStatusChangedEvent {
  charge_point_id: string;
  connector_id: number;
  status: string;
  error_code?: string;
  info?: string;
  timestamp: string;
}

interface MeterValuesEvent {
  charge_point_id: string;
  connector_id: number;
  transaction_id?: number;
  meter_value: number;
  unit: string;
  timestamp: string;
}
```

---

## Структура страниц

### 1. Login Page (`/login`)
- Форма: username + password
- Сохранить JWT в localStorage
- Redirect → Dashboard

### 2. Dashboard (`/`)
- **Карточки-счётчики** (данные из `/charge-points/stats`):
  - Всего станций
  - Онлайн
  - Заряжаются
  - Оффлайн
- **Карточки транзакций** (данные из `/transactions` и активные):
  - Активных транзакций
  - Завершённых за сегодня
  - Потреблено энергии (kWh)
- **Живая лента событий** (WebSocket) — последние 50 событий
- **Графики** (Recharts):
  - Транзакции за последние 7 дней (bar chart)
  - Энергопотребление (area chart)
  - Статусы станций (pie chart)

### 3. Charge Points (`/charge-points`)
- **Таблица** (TanStack Table): ID, вендор, модель, статус (badge), коннекторы, последний heartbeat
- **Фильтры**: по статусу (online/offline/charging), поиск по ID
- **Строка кликабельна** → переход на детальную страницу

### 4. Charge Point Detail (`/charge-points/:id`)
- **Информация**: вендор, модель, серийный номер, прошивка, статус
- **Коннекторы**: визуальные карточки с цветным статусом
- **Вкладки (Tabs)**:

#### Tab: Транзакции
- Таблица транзакций этой станции (пагинация, фильтры по статусу/датам)

#### Tab: Конфигурация ⭐ НОВОЕ
- Загрузить все ключи: `GET .../configuration` (без параметра `keys`)
- **⚠️ Если станция не поддерживает GetConfiguration** → API вернёт 500 → показать заглушку: "Станция не поддерживает чтение конфигурации" с кнопкой "Повторить"
- **Таблица**: ключ | значение | readonly (🔒/✏️)
- Inline-редактирование для не-readonly ключей: клик → input → Save → `PUT .../configuration`
- Индикация ответа:
  - ✅ `Accepted` — зелёный toast
  - ⚠️ `RebootRequired` — жёлтый toast "Требуется перезагрузка"
  - ❌ `Rejected` / `NotSupported` — красный toast
- Кнопка "Обновить конфигурацию" для перезагрузки списка

#### Tab: Команды
- **Кнопки-действия** (каждая открывает модалку/форму):
  - 🟢 Запустить зарядку → форма: id_tag (autocomplete из `/id-tags`), connector_id
  - 🔴 Остановить зарядку → форма: transaction_id (autocomplete из active transactions)
  - 🔄 Перезагрузить → выбор: Soft / Hard
  - 🔓 Разблокировать коннектор → connector_id
  - 🔧 Изменить доступность → connector_id (0 = вся станция) + Operative/Inoperative
  - 📨 Запросить сообщение → тип: StatusNotification/Heartbeat/MeterValues/...
  - 🗑️ Очистить кэш авторизации → подтверждение → `POST .../clear-cache`
  - 📋 Версия авт. списка → `GET .../local-list-version` → показать результат
  - 📡 Data Transfer → форма: vendor_id, message_id?, data?
- **Результат команды** → показать статус в toast или inline-блоке
- **Обработка ошибок**: при 500 от любой команды → красный toast с текстом из `error` (напр. "NotImplemented"). Не показывать как "Ошибка сервера" — это ответ станции

#### Tab: Мониторинг
- Live MeterValues (WebSocket, фильтр по этой станции)
- График потребления в реальном времени (Recharts, streaming)
- Heartbeat статус

### 5. Transactions (`/transactions`)
- **Таблица** (пагинация): ID, станция, коннектор, IdTag, статус (badge), энергия, время, причина остановки
- **Фильтры**: по станции, статусу (Active/Completed/Failed), датам
- **Force-stop кнопка** 🆕: для `Active` транзакций — красная кнопка "Принудительная остановка"
  - Подтверждение: "Эта операция не отправляет команду на станцию. Используйте Remote Stop если станция онлайн."
  - `POST /api/v1/transactions/{id}/force-stop`
  - Обновить строку таблицы после успеха

### 6. IdTags (`/id-tags`)
- **Таблица**: значение, статус (badge), пользователь, активен, срок действия, последнее использование
- **CRUD**: создание, редактирование (модалка), удаление (подтверждение)
- **Быстрые действия**: блокировать / разблокировать (toggle)

### 7. Tariffs (`/tariffs`)
- **Таблица**: название, тип, цена/kWh, цена/мин, стартовый сбор, валюта, по умолчанию (⭐)
- **CRUD**: создание/редактирование в модалке
- **Калькулятор стоимости**: форма → preview-cost → показать разбивку

### 8. Settings (`/settings`)
- **Профиль**: текущий пользователь, смена пароля
- **API Keys**: список, создание, отзыв
  - При создании: модалка с ключом + кнопка "Скопировать" + предупреждение "Ключ показывается один раз!"

---

## Layout

```
┌──────────────────────────────────────────────┐
│  🔌 Texnouz OCPP        [🔔 Events] [👤 User ▾]  │
├──────────┬───────────────────────────────────┤
│ Sidebar  │                                   │
│          │          Main Content              │
│ 📊 Dashboard │                               │
│ ⚡ Станции   │                               │
│ 📊 Транзакции│                               │
│ 🏷️ IdTags   │                               │
│ 💰 Тарифы   │                               │
│ ⚙️ Настройки│                               │
│          │                                   │
├──────────┴───────────────────────────────────┤
│  Status bar: 🟢 API Connected | 🟢 WS Connected │
└──────────────────────────────────────────────┘
```

- Sidebar: коллапсируемый, с иконками
- Тёмная тема по умолчанию (Catppuccin Mocha palette)
- Notification bell: количество непрочитанных WS-событий
- Status bar: состояние API + WebSocket подключений

---

## Ключевые требования

### API Client (`src/api/`)
```typescript
// api/client.ts — singleton Axios instance
const api = axios.create({
  baseURL: "http://localhost:8080/api/v1",
});

// Interceptor: добавлять Authorization header из Zustand store
api.interceptors.request.use((config) => {
  const token = useAuthStore.getState().token;
  if (token) config.headers.Authorization = `Bearer ${token}`;
  return config;
});

// Interceptor: при 401 — logout и redirect на /login
api.interceptors.response.use(
  (res) => res,
  (err) => {
    if (err.response?.status === 401) {
      useAuthStore.getState().logout();
      window.location.href = "/login";
    }
    return Promise.reject(err);
  }
);
```

### WebSocket (`src/hooks/useWebSocket.ts`)
```typescript
// Авто-реконнект с exponential backoff
// Парсинг event.data → WebSocketEvent
// Zustand store для последних N событий
// Фильтрация по charge_point_id и event types
// Показывать toast при критических событиях (error, transaction_stopped)
```

### React Query Hooks (`src/hooks/`)
```typescript
// Для каждого ресурса:
// - useChargePoints() → useQuery
// - useChargePoint(id) → useQuery
// - useChargePointConfig(id) → useQuery (GET .../configuration)
// - useChangeConfig(id) → useMutation (PUT .../configuration)
// - useLocalListVersion(id) → useQuery
// - useClearCache(id) → useMutation
// - useDataTransfer(id) → useMutation
// - useTransactions(filters) → useQuery (with pagination)
// - useForceStopTransaction() → useMutation
// - useIdTags(filters) → useQuery
// - useTariffs() → useQuery
// - usePreviewCost() → useMutation
// - useCommand(chargePointId) → useMutation (generic for all commands)
// - invalidateQueries on mutations
```

### State Management (Zustand)
```
stores/
  auth.ts       — token, user, login(), logout()
  events.ts     — WS events buffer (last 100), unread count
  ui.ts         — sidebar collapsed, active filters, theme
```

### Файловая структура
```
src/
  api/
    client.ts            — Axios instance + interceptors
    endpoints/
      auth.ts
      chargePoints.ts
      commands.ts        — все OCPP команды
      transactions.ts    — включая force-stop
      idTags.ts
      tariffs.ts
      monitoring.ts
      apiKeys.ts
  components/
    ui/                  — shadcn/ui components
    layout/
      Sidebar.tsx
      Header.tsx
      StatusBar.tsx
    dashboard/
      StatsCards.tsx
      EventFeed.tsx
      Charts.tsx
    charge-points/
      ChargePointTable.tsx
      ChargePointDetail.tsx
      ConfigurationTab.tsx    — ⭐ таблица конфигурации с inline-edit
      CommandsTab.tsx         — все 11 команд
      TransactionsTab.tsx
      MonitoringTab.tsx
    transactions/
      TransactionTable.tsx
      ForceStopButton.tsx     — ⭐ принудительная остановка
    id-tags/
      IdTagTable.tsx
      IdTagForm.tsx
    tariffs/
      TariffTable.tsx
      TariffForm.tsx
      CostCalculator.tsx
    settings/
      ProfileSection.tsx
      ApiKeysSection.tsx
  hooks/
    useWebSocket.ts
    useAuth.ts
    queries/               — React Query hooks
      useChargePoints.ts
      useCommands.ts
      useTransactions.ts
      useIdTags.ts
      useTariffs.ts
      useMonitoring.ts
  stores/
    auth.ts
    events.ts
    ui.ts
  pages/
    LoginPage.tsx
    DashboardPage.tsx
    ChargePointsPage.tsx
    ChargePointDetailPage.tsx
    TransactionsPage.tsx
    IdTagsPage.tsx
    TariffsPage.tsx
    SettingsPage.tsx
  types/
    api.ts               — все TypeScript типы из DTO
    events.ts            — WebSocket event types
  lib/
    utils.ts
    formatters.ts        — formatEnergy(), formatDuration(), formatCurrency()
  App.tsx
  main.tsx
  index.css              — Tailwind + тёмная тема
```

---

## UX детали

### Статусы станций (badges с цветами)
| Статус | Цвет | Иконка |
|--------|------|--------|
| Online / Available | 🟢 green | `Wifi` |
| Charging | 🔵 blue | `Zap` |
| Preparing / SuspendedEV | 🟡 yellow | `Clock` |
| Faulted | 🔴 red | `AlertTriangle` |
| Offline / Unavailable | ⚫ gray | `WifiOff` |

### Статусы транзакций
| Статус | Badge |
|--------|-------|
| Active | 🔵 синий пульсирующий |
| Completed | 🟢 зелёный |
| Failed | 🔴 красный |

### Статусы IdTag
| Статус | Badge |
|--------|-------|
| Accepted | 🟢 |
| Blocked | 🔴 |
| Expired | 🟡 |
| Invalid | ⚫ |

### Toast-уведомления
- Успех команды → зелёный toast
- RebootRequired → жёлтый toast с иконкой ⚠️
- Ошибка → красный toast
- WebSocket disconnect → persistent warning toast
- Новая транзакция → info toast

### Конфигурация станции (ConfigurationTab)
```
┌─────────────────────────────────────────────────┐
│ 🔧 Конфигурация OCPP           [🔄 Обновить]   │
├─────────────────┬──────────────┬────────────────┤
│ Ключ            │ Значение     │                │
├─────────────────┼──────────────┼────────────────┤
│ HeartbeatInterval│ 300         │ ✏️ Изменить    │
│ MeterValueSample│ 60          │ ✏️ Изменить    │
│ NumberOfConnect │ 2           │ 🔒 Только чтение│
│ ChargePointModel│ Texnouz-22K │ 🔒 Только чтение│
│ ...             │ ...         │                │
├─────────────────┴──────────────┴────────────────┤
│ Неизвестные ключи: (если есть)                  │
└─────────────────────────────────────────────────┘
```

- При клике "Изменить": inline-input + кнопки Save/Cancel
- После Save: показать результат (Accepted/RebootRequired/Rejected)
- Группировка ключей: Core, LocalAuthList, Metering, Charging, Connectivity

### Force Stop (TransactionTable)
```
┌──────┬─────────┬──────┬─────────┬─────────────────┐
│ ID   │ Станция │ Tag  │ Статус  │ Действия        │
├──────┼─────────┼──────┼─────────┼─────────────────┤
│ 42   │ CP001   │ RFID │ 🔵 Active│ [⛔ Force Stop] │
│ 41   │ CP001   │ RFID │ 🟢 Done │                 │
└──────┴─────────┴──────┴─────────┴─────────────────┘
```

При нажатии Force Stop:
1. Модалка подтверждения: "Транзакция #{id} будет принудительно закрыта в БД. Это НЕ отправит команду на станцию. Если станция онлайн — используйте Remote Stop."
2. `POST /api/v1/transactions/{id}/force-stop`
3. Обработка ответов:
   - 200 → обновить строку, зелёный toast
   - 404 → "Транзакция не найдена"
   - 409 → "Транзакция уже завершена"

---

## Дополнительно

- **Responsive**: sidebar коллапсируется на мобильных
- **Loading states**: скелетоны для таблиц, спиннеры для кнопок
- **Error boundaries**: красивые страницы ошибок
- **Empty states**: иллюстрации когда нет данных
- **Keyboard shortcuts**: Ctrl+K → поиск станции
- **Auto-refresh**: таблицы обновляются каждые 30 сек (React Query refetchInterval)
- **Все даты**: отображать в формате `DD.MM.YYYY HH:mm:ss` (русская локаль)
- **Энергия**: всегда показывать в kWh (делить Wh на 1000)
- **Валюта**: форматировать через `Intl.NumberFormat`
