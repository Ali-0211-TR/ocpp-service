//! Texnouz CSMS Desktop — Embedded Server Manager
//!
//! Wraps [`texnouz_csms::server::ServerHandle`] in a Tauri-friendly state
//! that supports start / stop / restart without spawning child processes.
//! The OCPP server runs in-process on the Tokio runtime.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::info;

use texnouz_csms::config::{default_config_path, AppConfig};
use texnouz_csms::server::{ServerHandle, ServerOptions};

// ── Inner state ────────────────────────────────────────────────────

struct Inner {
    handle: Option<ServerHandle>,
    config_path: PathBuf,
    config: AppConfig,
}

/// Thread-safe embedded server manager for the Tauri desktop app.
///
/// Stored as Tauri managed state via `app.manage(EmbeddedServer::new())`.
#[derive(Clone)]
pub struct EmbeddedServer {
    inner: Arc<Mutex<Inner>>,
}

impl EmbeddedServer {
    /// Create a new server manager. Does **not** start the server yet.
    pub fn new() -> Self {
        let config_path = default_config_path();
        let config = AppConfig::load(&config_path).unwrap_or_default();
        Self {
            inner: Arc::new(Mutex::new(Inner {
                handle: None,
                config_path,
                config,
            })),
        }
    }

    /// Start the embedded OCPP server.
    pub async fn start(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        if inner.handle.is_some() {
            return Err("Сервер уже запущен".into());
        }

        info!("[desktop] Starting embedded OCPP server...");

        // Reload config from disk
        if let Ok(cfg) = AppConfig::load(&inner.config_path) {
            inner.config = cfg;
        }

        // For the desktop app, if SQLite path is relative (e.g. "./ocpp.db"),
        // resolve it next to the config file so it doesn't land in the CWD
        // (which is src-tauri/ during development and triggers rebuild loops).
        if inner.config.database.driver == texnouz_csms::config::DbType::Sqlite {
            let db_path = std::path::Path::new(&inner.config.database.sqlite.path);
            if db_path.is_relative() {
                if let Some(config_dir) = inner.config_path.parent() {
                    let resolved = config_dir.join(db_path);
                    inner.config.database.sqlite.path = resolved.to_string_lossy().into_owned();
                    info!("[desktop] SQLite path resolved to {}", inner.config.database.sqlite.path);
                }
            }
        }

        let handle = ServerHandle::start(ServerOptions {
            config: inner.config.clone(),
            auto_migrate: true,
            create_default_admin: true,
        })
        .await
        .map_err(|e| format!("Не удалось запустить сервер: {e}"))?;

        info!(
            "[desktop] Server started (API :{}, WS :{})",
            handle.api_port, handle.ws_port
        );

        inner.handle = Some(handle);
        Ok(())
    }

    /// Stop the embedded OCPP server gracefully.
    pub async fn stop(&self) {
        let mut inner = self.inner.lock().await;
        if let Some(handle) = inner.handle.take() {
            info!("[desktop] Stopping embedded OCPP server...");
            handle.shutdown().await;
            info!("[desktop] Server stopped");
        }
    }

    /// Restart the server (stop + reload config + start).
    pub async fn restart(&self) -> Result<(), String> {
        self.stop().await;
        // Small delay for TCP sockets to release (TIME_WAIT)
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        self.start().await
    }

    /// Check if the server is currently running.
    pub fn is_running(&self) -> bool {
        // try_lock to avoid blocking from sync context (tray menu)
        match self.inner.try_lock() {
            Ok(inner) => inner.handle.is_some(),
            Err(_) => false, // locked means likely in transition
        }
    }

    /// Get the configured API port.
    pub fn api_port(&self) -> u16 {
        match self.inner.try_lock() {
            Ok(inner) => inner.config.server.api_port,
            Err(_) => 3000,
        }
    }

    /// Get the configured WebSocket port.
    pub fn ws_port(&self) -> u16 {
        match self.inner.try_lock() {
            Ok(inner) => inner.config.server.ws_port,
            Err(_) => 9000,
        }
    }

    /// Get the config path.
    pub fn config_path(&self) -> PathBuf {
        match self.inner.try_lock() {
            Ok(inner) => inner.config_path.clone(),
            Err(_) => default_config_path(),
        }
    }

    /// Get a clone of the current config.
    pub fn config(&self) -> AppConfig {
        match self.inner.try_lock() {
            Ok(inner) => inner.config.clone(),
            Err(_) => AppConfig::default(),
        }
    }

    /// Save config to disk and update in-memory state.
    pub async fn save_config(&self, config: AppConfig) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        let path = inner.config_path.clone();
        config.save(&path)?;
        info!("[desktop] Config saved to {}", path.display());
        inner.config = config;
        Ok(())
    }

    /// Get a reference to the inner Arc for async spawning.
    pub fn inner(&self) -> &EmbeddedServer {
        self
    }
}
