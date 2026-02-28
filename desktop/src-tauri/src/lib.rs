//! Texnouz CSMS Desktop — Tauri Application
//!
//! Full-featured desktop CSMS management application:
//! - **System tray** with server controls (start / stop / restart)
//! - **Embedded OCPP server** running in-process (no child process spawning)
//! - **React dashboard** for charge point management, transactions, analytics
//! - **Desktop notifications** for critical events (station offline, errors)
//!
//! The embedded server uses [`texnouz_csms`] library directly, providing
//! instant access to all server state (sessions, events, repositories).

pub mod commands;
pub mod server;
pub mod tray;

use tracing::info;

use crate::server::EmbeddedServer;

/// Entry point for the Tauri application.
pub fn run() {
    // Initialize tracing for the desktop app
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let embedded_server = EmbeddedServer::new();

    tauri::Builder::default()
        .manage(embedded_server.clone())
        .setup(move |app| {
            // Auto-start the embedded OCPP server
            let srv = embedded_server.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = srv.start().await {
                    tracing::error!("[desktop] Auto-start failed: {e}");
                }
            });

            // Setup system tray
            tray::setup_tray(app)?;

            info!("[desktop] Texnouz CSMS Desktop started");
            info!("[desktop] Config: {}", embedded_server.config_path().display());

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide window on close instead of destroying (stay in tray)
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::server_status,
            commands::server_start,
            commands::server_stop,
            commands::server_restart,
            commands::get_config,
            commands::save_config,
            commands::save_and_restart,
            commands::get_config_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Texnouz CSMS Desktop");
}
