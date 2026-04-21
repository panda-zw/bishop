//! Dynamic system-tray menu.
//!
//! Structure (rebuilt on refresh_tray):
//!
//!   Bishop ▪ <status summary>
//!   ───────────────────────────
//!   Projects
//!     sample-app     ▸  staging
//!                       production
//!     other-app      ▸  staging
//!   ───────────────────────────
//!   Saved hosts
//!     dev-vps
//!     staging-box
//!   ───────────────────────────
//!   SSH config
//!     production
//!     bastion
//!   ───────────────────────────
//!   Show Bishop
//!   Hide
//!   Quit  (⌘Q)
//!
//! Item IDs encode the action so `on_menu_event` can dispatch:
//!   show / hide / quit                                 — handled in Rust
//!   open-project:<path>                                — emit event
//!   open-env:<path>::<env>                             — emit event
//!   connect-host:<id>                                  — emit event (saved host)
//!   connect-alias:<alias>                              — emit event (ssh_config)

use crate::commands::hosts;
use crate::config::{project, store};
use std::path::PathBuf;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, Submenu, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

/// Source PNG for the tray icon — embedded at compile time so runtime has no
/// filesystem dependency. Same art as the app icon; the rounding/resize pass
/// below makes it read well at macOS menubar sizes (18–22 px).
const TRAY_ICON_PNG: &[u8] = include_bytes!("../icons/128x128.png");

/// Resize + circular-ish mask. The corner radius is ~42% of the side which
/// reads as near-circular at 22 px (menubar) and still shows a hint of square
/// at larger zoom levels (Windows/Linux where tray icons can be 32+ px).
fn rounded_tray_icon() -> Result<Image<'static>, Box<dyn std::error::Error>> {
    const TARGET: u32 = 44;
    const RADIUS_PCT: f32 = 0.42;

    let decoded = image::load_from_memory_with_format(TRAY_ICON_PNG, image::ImageFormat::Png)?
        .resize_exact(TARGET, TARGET, image::imageops::FilterType::Lanczos3)
        .to_rgba8();

    let mut pixels = decoded.into_raw();
    let radius = (TARGET as f32 * RADIUS_PCT).round();
    let r2 = radius * radius;

    // Squared-distance test against the four corner circles. Pixels outside
    // the rounded-rect union have alpha zeroed; interior pixels keep their
    // original alpha so anti-aliased edges of the source PNG still blend.
    for y in 0..TARGET {
        for x in 0..TARGET {
            let (fx, fy) = (x as f32, y as f32);
            let (far_x, far_y) = ((TARGET - 1 - x) as f32, (TARGET - 1 - y) as f32);

            let cx = if fx < radius { radius - fx } else if far_x < radius { radius - far_x } else { 0.0 };
            let cy = if fy < radius { radius - fy } else if far_y < radius { radius - far_y } else { 0.0 };

            if cx > 0.0 && cy > 0.0 {
                let d2 = cx * cx + cy * cy;
                let idx = ((y * TARGET + x) * 4 + 3) as usize;
                if d2 > r2 {
                    pixels[idx] = 0;
                } else {
                    // One-pixel band near the edge gets softened so the mask
                    // doesn't produce visible stair-step aliasing.
                    let d = d2.sqrt();
                    if d > radius - 1.0 {
                        let t = (radius - d).clamp(0.0, 1.0);
                        pixels[idx] = ((pixels[idx] as f32) * t) as u8;
                    }
                }
            }
        }
    }

    Ok(Image::new_owned(pixels, TARGET, TARGET))
}

/// Build the tray on app setup. Registers a tray with id "main" and initial menu.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    // Fall back to the window icon if rounding fails — better a square icon
    // than no tray at all.
    let icon = rounded_tray_icon()
        .unwrap_or_else(|_| app.default_window_icon().cloned().unwrap());

    TrayIconBuilder::with_id("main")
        .tooltip("Bishop")
        .icon(icon)
        .menu(&menu)
        .on_menu_event(|app, event| dispatch_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Tauri command — frontend can ask us to rebuild the menu after projects or
/// saved hosts change.
#[tauri::command]
pub fn refresh_tray(app: AppHandle) -> Result<(), String> {
    let menu = build_menu(&app).map_err(|e| format!("{}", e))?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu)).map_err(|e| format!("{}", e))?;
    }
    Ok(())
}

/// Tauri command — frontend can update the tooltip with a live status summary.
#[tauri::command]
pub fn set_tray_tooltip(app: AppHandle, tooltip: String) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_tooltip(Some(tooltip)).map_err(|e| format!("{}", e))?;
    }
    Ok(())
}

fn dispatch_event(app: &AppHandle, id: &str) {
    match id {
        "show" => show_window(app),
        "hide" => hide_window(app),
        "quit" => app.exit(0),
        _ if id.starts_with("open-project:") => {
            let path = &id["open-project:".len()..];
            show_window(app);
            let _ = app.emit("tray:open-project", path.to_string());
        }
        _ if id.starts_with("open-env:") => {
            let rest = &id["open-env:".len()..];
            if let Some((path, env)) = rest.split_once("::") {
                show_window(app);
                let _ = app.emit("tray:open-env", serde_json::json!({
                    "project_path": path, "env": env,
                }));
            }
        }
        _ if id.starts_with("connect-host:") => {
            let host_id = &id["connect-host:".len()..];
            show_window(app);
            let _ = app.emit("tray:connect-host", host_id.to_string());
        }
        _ if id.starts_with("connect-alias:") => {
            let alias = &id["connect-alias:".len()..];
            show_window(app);
            let _ = app.emit("tray:connect-alias", alias.to_string());
        }
        _ => { /* unknown id — ignore */ }
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;

    // ---- Projects (grouped submenus) ----
    let settings = store::load();
    let mut has_projects = false;
    for path in &settings.project_paths {
        let Ok(proj) = project::load_project(&PathBuf::from(path)) else { continue };
        has_projects = true;
        let project_sub = Submenu::new(app, &proj.name, true)?;
        // First item: open project in window
        let open_item = MenuItem::with_id(
            app,
            format!("open-project:{}", path),
            "Open in Bishop",
            true,
            None::<&str>,
        )?;
        project_sub.append(&open_item)?;
        project_sub.append(&PredefinedMenuItem::separator(app)?)?;

        for env in &proj.remotes {
            let item = MenuItem::with_id(
                app,
                format!("open-env:{}::{}", path, env.name),
                &format!("{}  —  {}@{}", env.name, env.ssh_user, env.ssh_host),
                true,
                None::<&str>,
            )?;
            project_sub.append(&item)?;
        }
        menu.append(&project_sub)?;
    }
    if !has_projects {
        let none = MenuItem::with_id(app, "noop-projects", "No projects yet", false, None::<&str>)?;
        menu.append(&none)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    // ---- Saved hosts ----
    let saved = settings.saved_hosts;
    if !saved.is_empty() {
        let sub = Submenu::new(app, "Saved hosts", true)?;
        for host in &saved {
            let label = format!("{}  —  {}@{}", host.label, host.user, host.host);
            let item = MenuItem::with_id(
                app,
                format!("connect-host:{}", host.id),
                &label,
                true,
                None::<&str>,
            )?;
            sub.append(&item)?;
        }
        menu.append(&sub)?;
    }

    // ---- SSH config ----
    let ssh_hosts = hosts::list_ssh_config_hosts();
    if !ssh_hosts.is_empty() {
        let sub = Submenu::new(app, "SSH config", true)?;
        for h in ssh_hosts.iter().take(50) {
            let item = MenuItem::with_id(
                app,
                format!("connect-alias:{}", h.alias),
                &h.alias,
                true,
                None::<&str>,
            )?;
            sub.append(&item)?;
        }
        menu.append(&sub)?;
    }

    if !saved.is_empty() || !ssh_hosts.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }

    // ---- Window controls ----
    let show = MenuItem::with_id(app, "show", "Show Bishop", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Bishop", true, Some("CmdOrCtrl+Q"))?;
    menu.append(&show)?;
    menu.append(&hide)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&quit)?;

    Ok(menu)
}

fn show_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.unminimize();
    }
}

fn hide_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

fn toggle_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        match win.is_visible() {
            Ok(true) => { let _ = win.hide(); }
            _ => {
                let _ = win.show();
                let _ = win.set_focus();
                let _ = win.unminimize();
            }
        }
    }
}
