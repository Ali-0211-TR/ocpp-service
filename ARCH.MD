```
src/
├── support/                         # Cross-cutting concerns
│   ├── mod.rs
│   ├── errors.rs                    # DomainError, InfraError, AppError
│   ├── validation.rs
│   ├── serialization.rs
│   ├── time.rs                      # Clock trait
│   └── id.rs
│
├── domain/
│   ├── models/                      # Чистые бизнес-модели
│   ├── repositories/                # Trait-ы (порты)
│   ├── services/                    # Domain service traits (AuthService, BillingService)
│   └── mod.rs
│
├── application/
│   ├── commands/                    # OCPP commands
│   ├── handlers/                    # OCPP message handlers
│   ├── services/                    # Реализации domain service traits
│   ├── session/                     # Connection registry + message dispatch
│   │   ├── connection.rs
│   │   ├── registry.rs
│   │   └── mod.rs
│   ├── dto/                         # Shared DTO
│   └── mod.rs
│
├── infrastructure/
│   ├── database/
│   │   ├── entities/
│   │   ├── migrator/
│   │   ├── repositories/            # SeaORM impl of domain repo traits
│   │   └── storage.rs
│   ├── crypto/                      # Примитивы: JWT, password, API key
│   │   ├── jwt.rs
│   │   ├── password.rs
│   │   └── api_key.rs
│   └── mod.rs
│
├── interfaces/
│   ├── http/                        # REST API
│   │   ├── router.rs
│   │   ├── state.rs
│   │   ├── middleware/
│   │   ├── handlers/
│   │   └── dto/
│   ├── ws/
│   │   ├── ocpp/                    # OCPP WS server
│   │   └── notifications/           # Client WS notifications
│   ├── grpc/                        # Заготовка
│   └── mod.rs
│
├── config.rs
├── lib.rs
└── main.rs
```