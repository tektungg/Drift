use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

pub fn install(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let show = MenuItem::with_id(app, "show", "Show Drift", true, None::<&str>)?;
    let pause_all = MenuItem::with_id(app, "pause_all", "Pause all", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu: Menu<Wry> = Menu::with_items(app, &[&show, &pause_all, &quit])?;

    TrayIconBuilder::with_id("drift-tray")
        .menu(&menu)
        // Menu opens on RIGHT-click only; left-click is handled by
        // on_tray_icon_event below (toggle window visibility).
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "pause_all" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let ctx = handle.state::<crate::commands::AppCtx>();
                    let snap = ctx.state.snapshot();
                    for mut r in snap.torrents {
                        if ctx.engine.pause(&crate::magnet::InfoHash(r.infohash.clone())).await.is_ok() {
                            r.state = crate::state::TorrentState::Paused;
                            let _ = ctx.state.upsert(r);
                        }
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            // Only toggle on a real left-click release. Without this filter, Tauri's
            // Enter/Move/Leave events would flicker the window every time the user's
            // cursor passes over the tray icon.
            if !matches!(
                ev,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                return;
            }
            if let Some(w) = tray.app_handle().get_webview_window("main") {
                if w.is_visible().unwrap_or(false) {
                    let _ = w.hide();
                } else {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .icon(app.default_window_icon().unwrap().clone())
        .build(app)
}
