//! System tray icon + menu.

use std::sync::Arc;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::core::launch;
use crate::engine::{Engine, Status};

pub const TRAY_ID: &str = "main";
pub const EVENT_NAVIGATE: &str = "navigate";

/// Handles kept so settings changes from the UI can update the tray.
pub struct TrayState {
    pub enabled: CheckMenuItem<Wry>,
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn navigate(app: &AppHandle, route: &str) {
    show_main(app);
    let _ = app.emit(EVENT_NAVIGATE, route);
}

pub fn tooltip(status: &Status) -> String {
    let mut t = format!("Veil Launcher: {}", status.phase.label());
    if let Some(target) = &status.target
        && status.swapped
    {
        t.push_str(&format!("\nCustom video on {target}"));
    }
    t
}

pub fn set_tooltip(app: &AppHandle, status: &Status) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tooltip(status)));
    }
}

pub fn set_enabled_checked(app: &AppHandle, enabled: bool) {
    if let Some(t) = app.try_state::<TrayState>() {
        let _ = t.enabled.set_checked(enabled);
    }
}

fn engine(app: &AppHandle) -> Arc<Engine> {
    app.state::<Arc<Engine>>().inner().clone()
}

/// Run `f` off the UI thread.
fn bg(app: &AppHandle, f: impl FnOnce(AppHandle, Arc<Engine>) + Send + 'static) {
    let app = app.clone();
    let engine = engine(&app);
    std::thread::spawn(move || f(app, engine));
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "launch" => bg(app, |_, e| {
            let rc = e.install_info().riot_client;
            if let Err(err) = launch::launch_valorant(rc.as_deref()) {
                e.notify_error(format!("{err:#}"));
            }
        }),
        "change_video" => navigate(app, "videos"),
        "toggle_enabled" => {
            let checked = app
                .try_state::<TrayState>()
                .and_then(|t| t.enabled.is_checked().ok())
                .unwrap_or(true);
            bg(app, move |app, e| {
                if let Err(err) = e.update_settings(&serde_json::json!({ "enabled": checked })) {
                    e.notify_error(format!("{err:#}"));
                    set_enabled_checked(&app, e.settings().enabled);
                }
            });
        }
        "restore" => bg(app, |_, e| {
            let _ = e.restore_now(true);
        }),
        "open" => show_main(app),
        "settings" => navigate(app, "settings"),
        "quit" => bg(app, |app, e| {
            e.prepare_quit();
            app.exit(0);
        }),
        _ => {}
    }
}

pub fn build(app: &AppHandle, engine: &Arc<Engine>) -> tauri::Result<()> {
    let launch = MenuItem::with_id(app, "launch", "Launch VALORANT", true, None::<&str>)?;
    let change = MenuItem::with_id(
        app,
        "change_video",
        "Change video\u{2026}",
        true,
        None::<&str>,
    )?;
    let enabled = CheckMenuItem::with_id(
        app,
        "toggle_enabled",
        "Custom background",
        true,
        engine.settings().enabled,
        None::<&str>,
    )?;
    let restore = MenuItem::with_id(app, "restore", "Restore original now", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let open = MenuItem::with_id(app, "open", "Open Veil Launcher", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &launch, &change, &enabled, &restore, &sep1, &open, &settings, &sep2, &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tooltip(&engine.status()))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().0.as_str()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.manage(TrayState { enabled });
    Ok(())
}
