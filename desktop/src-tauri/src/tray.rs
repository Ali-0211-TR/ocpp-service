//! Texnouz CSMS Desktop — System Tray module
//!
//! Builds the tray icon menu and handles menu events (start/stop/restart,
//! open dashboard, quit).

use tauri::{
    image::Image,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Runtime,
};

use crate::server::EmbeddedServer;

// ── Build tray menu ────────────────────────────────────────────────

pub fn build_tray_menu<R: Runtime>(
    app: &impl Manager<R>,
    running: bool,
) -> tauri::Result<Menu<R>> {
    let status_text = if running {
        "🟢 Texnouz CSMS — Работает"
    } else {
        "🔴 Texnouz CSMS — Остановлен"
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
            &MenuItem::with_id(app, "dashboard", "📊  Панель управления", true, None::<&str>)?,
            &MenuItem::with_id(app, "settings", "⚙️  Настройки", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "open_swagger", "📖  Swagger UI", running, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "❌  Выйти", true, None::<&str>)?,
        ],
    )
}

// ── Refresh tray state ─────────────────────────────────────────────

pub fn refresh_tray<R: Runtime>(app: &AppHandle<R>) {
    let server = app.state::<EmbeddedServer>();
    let running = server.is_running();

    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_tray_menu(app, running) {
            let _ = tray.set_menu(Some(menu));
        }
        let tooltip = if running {
            "Texnouz CSMS — Работает"
        } else {
            "Texnouz CSMS — Остановлен"
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

// ── Handle tray menu events ───────────────────────────────────────

pub fn handle_tray_menu<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let server = app.state::<EmbeddedServer>();

    match event.id.as_ref() {
        "start" => {
            let handle = app.clone();
            let srv = server.inner().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = srv.start().await {
                    tracing::warn!("[desktop] Start requested but: {e}");
                }
                refresh_tray(&handle);
            });
        }
        "stop" => {
            let handle = app.clone();
            let srv = server.inner().clone();
            tauri::async_runtime::spawn(async move {
                srv.stop().await;
                refresh_tray(&handle);
            });
        }
        "restart" => {
            let handle = app.clone();
            let srv = server.inner().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = srv.restart().await {
                    tracing::error!("[desktop] Restart failed: {e}");
                }
                refresh_tray(&handle);
            });
        }
        "dashboard" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "settings" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                // Navigate to settings page via event
                let _ = window.emit("navigate", "/settings");
            }
        }
        "open_swagger" => {
            let port = server.api_port();
            let url = format!("http://localhost:{}/docs/", port);
            // Open in default browser
            let _ = std::process::Command::new("xdg-open")
                .arg(&url)
                .spawn();
        }
        "quit" => {
            let srv = server.inner().clone();
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                srv.stop().await;
                handle.exit(0);
            });
        }
        _ => {}
    }
}

// ── Setup tray icon ────────────────────────────────────────────────

pub fn setup_tray<R: Runtime>(app: &mut tauri::App<R>) -> tauri::Result<()> {
    let server = app.state::<EmbeddedServer>();
    let running = server.is_running();
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
            "Texnouz CSMS — Работает"
        } else {
            "Texnouz CSMS — Остановлен"
        })
        .on_menu_event(|app, event| {
            handle_tray_menu(app, event);
        })
        .build(app)?;

    Ok(())
}
