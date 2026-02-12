//! Texnouz OCPP Desktop — Tauri application
//!
//! System tray with server controls (start / stop / restart),
//! settings window (web frontend) and Web UI launcher.

use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;

use tauri::{
    image::Image,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Runtime, State,
};

use texnouz_ocpp::config::{default_config_path, AppConfig};

// ── Server State ───────────────────────────────────────────────

pub struct ServerState {
    inner: Mutex<ServerStateInner>,
}

struct ServerStateInner {
    process: Option<Child>,
    config_path: PathBuf,
    config: AppConfig,
}

impl ServerState {
    pub fn new() -> Self {
        let config_path = default_config_path();
        let config = AppConfig::load(&config_path).unwrap_or_default();
        Self {
            inner: Mutex::new(ServerStateInner {
                process: None,
                config_path,
                config,
            }),
        }
    }

    pub fn is_running(&self) -> bool {
        let mut s = self.inner.lock().unwrap();
        if let Some(ref mut child) = s.process {
            match child.try_wait() {
                Ok(Some(_)) => {
                    s.process = None;
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    s.process = None;
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn start_server(&self) -> Result<(), String> {
        let mut s = self.inner.lock().unwrap();
        if s.process.is_some() {
            return Err("Сервер уже запущен".into());
        }

        let exe = std::env::current_exe().unwrap_or_default();
        let server_exe = exe
            .parent()
            .map(|p| p.join("ocpp-service"))
            .unwrap_or_else(|| PathBuf::from("ocpp-service"));

        // Pass config path explicitly via env var
        let config_path = s.config_path.clone();

        let child = Command::new(&server_exe)
            .env("OCPP_CONFIG", &config_path)
            .spawn()
            .map_err(|e| format!("Не удалось запустить сервер: {e}"))?;

        eprintln!(
            "[desktop] Server started (PID {}) config={}",
            child.id(),
            config_path.display()
        );
        s.process = Some(child);
        Ok(())
    }

    pub fn stop_server(&self) -> Result<(), String> {
        let mut s = self.inner.lock().unwrap();
        if let Some(ref mut child) = s.process {
            let pid = child.id();
            eprintln!("[desktop] Stopping server (PID {pid})...");

            // Send SIGTERM on Unix, kill on Windows
            #[cfg(unix)]
            {
                let _ = Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .status();
            }
            #[cfg(not(unix))]
            {
                let _ = child.kill();
            }

            // Wait for process to exit (with timeout)
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        eprintln!("[desktop] Server exited ({status})");
                        break;
                    }
                    Ok(None) => {
                        if std::time::Instant::now() >= deadline {
                            eprintln!("[desktop] Server did not exit in time, killing...");
                            let _ = child.kill();
                            let _ = child.wait();
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(_) => break,
                }
            }
        }
        s.process = None;
        Ok(())
    }

    pub fn restart_server(&self) -> Result<(), String> {
        self.stop_server()?;

        // Wait for TCP sockets to fully release (TIME_WAIT)
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Reload config from disk so new port/settings take effect
        {
            let mut s = self.inner.lock().unwrap();
            if let Ok(cfg) = AppConfig::load(&s.config_path) {
                eprintln!(
                    "[desktop] Config reloaded (api_port={}, ws_port={})",
                    cfg.server.api_port, cfg.server.ws_port
                );
                s.config = cfg;
            }
        }
        self.start_server()
    }

    pub fn api_port(&self) -> u16 {
        self.inner.lock().unwrap().config.server.api_port
    }

    pub fn config_path(&self) -> PathBuf {
        self.inner.lock().unwrap().config_path.clone()
    }
}

// ── Tauri Commands ─────────────────────────────────────────────

#[tauri::command]
fn get_config(state: State<'_, ServerState>) -> Result<AppConfig, String> {
    let s = state.inner.lock().map_err(|e| e.to_string())?;
    Ok(s.config.clone())
}

#[tauri::command]
fn save_config(state: State<'_, ServerState>, config: AppConfig) -> Result<(), String> {
    let mut s = state.inner.lock().map_err(|e| e.to_string())?;
    let path = s.config_path.clone();
    config.save(&path)?;
    eprintln!("[desktop] Config saved to {}", path.display());
    s.config = config;
    Ok(())
}

#[tauri::command]
fn server_start(state: State<'_, ServerState>) -> Result<(), String> {
    state.start_server()
}

#[tauri::command]
fn server_stop(state: State<'_, ServerState>) -> Result<(), String> {
    state.stop_server()
}

#[tauri::command]
fn server_restart(state: State<'_, ServerState>) -> Result<(), String> {
    state.restart_server()
}

#[tauri::command]
fn save_and_restart(state: State<'_, ServerState>, config: AppConfig) -> Result<(), String> {
    // Save config to disk
    {
        let mut s = state.inner.lock().map_err(|e| e.to_string())?;
        let path = s.config_path.clone();
        config.save(&path)?;
        eprintln!("[desktop] Config saved to {}", path.display());
        s.config = config;
    }
    // Restart server with new config
    state.restart_server()
}

#[tauri::command]
fn server_status(state: State<'_, ServerState>) -> bool {
    state.is_running()
}

#[tauri::command]
fn get_config_path(state: State<'_, ServerState>) -> String {
    state.config_path().display().to_string()
}

// ── Tray Menu ──────────────────────────────────────────────────

fn build_tray_menu<R: Runtime>(app: &impl Manager<R>, running: bool) -> tauri::Result<Menu<R>> {
    let status_text = if running {
        "🟢 Texnouz OCPP — Работает"
    } else {
        "🔴 Texnouz OCPP — Остановлен"
    };

    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "status", status_text, false, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "start", "▶️  Запустить", !running, None::<&str>)?,
            &MenuItem::with_id(app, "stop", "⏹  Остановить", running, None::<&str>)?,
            &MenuItem::with_id(app, "restart", "🔄  Перезапустить", running, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "settings", "⚙️  Настройки", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "❌  Выйти", true, None::<&str>)?,
        ],
    )
}

fn refresh_tray<R: Runtime>(app: &AppHandle<R>, state: &ServerState) {
    if let Some(tray) = app.tray_by_id("main") {
        let running = state.is_running();
        if let Ok(menu) = build_tray_menu(app, running) {
            let _ = tray.set_menu(Some(menu));
        }
        let tooltip = if running {
            "Texnouz OCPP — Работает"
        } else {
            "Texnouz OCPP — Остановлен"
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

fn handle_tray_menu<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let state = app.state::<ServerState>();

    match event.id.as_ref() {
        "start" => {
            let _ = state.start_server();
        }
        "stop" => {
            let _ = state.stop_server();
        }
        "restart" => {
            let _ = state.restart_server();
        }
        "settings" => {
            if let Some(window) = app.get_webview_window("settings") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "quit" => {
            let _ = state.stop_server();
            app.exit(0);
        }
        _ => {}
    }

    refresh_tray(app, &state);
}

// ── App Entry ──────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .manage(ServerState::new())
        .setup(|app| {
            let state = app.state::<ServerState>();

            // Auto-start server
            let _ = state.start_server();
            let running = state.is_running();

            let handle = app.handle();
            let menu = build_tray_menu(handle, running)?;

            let icon = {
                let img = image::load_from_memory(include_bytes!("../icons/icon.png"))
                    .expect("Failed to decode tray icon");
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                Image::new_owned(rgba.into_raw(), w, h)
            };

            TrayIconBuilder::with_id("main")
                .icon(icon)
                .menu(&menu)
                .tooltip(if running {
                    "Texnouz OCPP — Работает"
                } else {
                    "Texnouz OCPP — Остановлен"
                })
                .on_menu_event(|app, event| {
                    handle_tray_menu(app, event);
                })
                .build(app)?;

            eprintln!("[desktop] Texnouz OCPP tray started");
            eprintln!("[desktop] Config: {}", state.config_path().display());

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide settings window on close instead of destroying it (stay in tray)
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            save_and_restart,
            server_start,
            server_stop,
            server_restart,
            server_status,
            get_config_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Texnouz OCPP desktop app");
}
