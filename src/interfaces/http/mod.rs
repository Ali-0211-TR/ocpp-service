//! HTTP REST API interfaces
//!
//! Organized by feature (aggregate-based grouping):
//! each sub-module contains its own DTOs, handlers, and state.
//!
//! - `common`        — shared DTOs (ApiResponse, PaginatedResponse, etc.)
//! - `middleware`     — authentication middleware (JWT + API key)
//! - `router`        — API router with Swagger documentation
//! - `auth`          — login, register, profile, password change
//! - `users`         — user CRUD (admin management)
//! - `api_keys`      — API key management
//! - `charge_points` — charge point CRUD + connectors
//! - `commands`      — OCPP remote commands
//! - `transactions`  — charging session management
//! - `tariffs`       — tariff management
//! - `id_tags`       — RFID tag management
//! - `monitoring`    — heartbeat & connection stats
//! - `health`        — health check

// ── Shared ──────────────────────────────────────────────────────
pub mod common;
pub mod middleware;
pub mod router;

// ── Feature modules ─────────────────────────────────────────────
pub mod modules;

// ── Convenience re-exports ──────────────────────────────────────
pub use common::*;
pub use router::create_api_router;