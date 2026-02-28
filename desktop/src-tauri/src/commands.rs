//! Texnouz CSMS Desktop — Tauri IPC Commands
//!
//! Commands exposed to the React frontend via `@tauri-apps/api/core`.
//! Server lifecycle commands use the embedded server; data commands
//! are thin wrappers that call the local REST API.

use tauri::State;

use crate::server::EmbeddedServer;
use texnouz_csms::config::AppConfig;

// ── Server lifecycle ───────────────────────────────────────────────

/// Get current server status.
#[tauri::command]
pub fn server_status(state: State<'_, EmbeddedServer>) -> ServerInfo {
    ServerInfo {
        running: state.is_running(),
        api_port: state.api_port(),
        ws_port: state.ws_port(),
        config_path: state.config_path().display().to_string(),
    }
}

/// Start the OCPP server.
#[tauri::command]
pub async fn server_start(state: State<'_, EmbeddedServer>) -> Result<(), String> {
    state.start().await
}

/// Stop the OCPP server.
#[tauri::command]
pub async fn server_stop(state: State<'_, EmbeddedServer>) -> Result<(), String> {
    state.stop().await;
    Ok(())
}

/// Restart the OCPP server (stop → reload config → start).
#[tauri::command]
pub async fn server_restart(state: State<'_, EmbeddedServer>) -> Result<(), String> {
    state.restart().await
}

// ── Configuration ──────────────────────────────────────────────────

/// Get the current server configuration.
#[tauri::command]
pub fn get_config(state: State<'_, EmbeddedServer>) -> AppConfig {
    state.config()
}

/// Save configuration to disk.
#[tauri::command]
pub async fn save_config(
    state: State<'_, EmbeddedServer>,
    config: AppConfig,
) -> Result<(), String> {
    state.save_config(config).await
}

/// Save configuration and restart the server to apply changes.
#[tauri::command]
pub async fn save_and_restart(
    state: State<'_, EmbeddedServer>,
    config: AppConfig,
) -> Result<(), String> {
    state.save_config(config).await?;
    state.restart().await
}

/// Get config file path.
#[tauri::command]
pub fn get_config_path(state: State<'_, EmbeddedServer>) -> String {
    state.config_path().display().to_string()
}

// ── Types ──────────────────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
pub struct ServerInfo {
    pub running: bool,
    pub api_port: u16,
    pub ws_port: u16,
    pub config_path: String,
}
