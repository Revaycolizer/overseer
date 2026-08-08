use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};

/// Show the window if hidden, hide it if visible.
fn toggle_window(app: &tauri::AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Single-instance MUST be registered first: a second launch (e.g. autostart +
    // a manual run) just summons the existing window instead of starting again and
    // colliding on the global hotkey.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            for label in ["widget", "main"] {
                if let Some(w) = app.get_webview_window(label) {
                    let _ = w.show();
                    let _ = w.set_focus();
                    break;
                }
            }
        }));
    }

    builder = builder.plugin(tauri_plugin_opener::init());

    #[cfg(desktop)]
    {
        use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};
        // Ctrl+Alt+Space summons / dismisses Overseer from anywhere.
        let toggle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
        builder = builder
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ))
            .plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcut(toggle)
                    .expect("register global shortcut")
                    .with_handler(|app, _shortcut, event| {
                        if event.state == ShortcutState::Pressed {
                            // Summon/dismiss the floating widget (the always-on assistant).
                            toggle_window(app, "widget");
                        }
                    })
                    .build(),
            );
    }

    builder
        .setup(|app| {
            #[cfg(desktop)]
            {
                // Launch on login so Overseer is always resident and summonable.
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();

                let widget = MenuItem::with_id(app, "widget", "Assistant Widget", true, None::<&str>)?;
                let show = MenuItem::with_id(app, "show", "Open Full App", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&widget, &show, &quit])?;
                let _tray = TrayIconBuilder::new()
                    .icon(app.default_window_icon().unwrap().clone())
                    .tooltip("Overseer")
                    .menu(&menu)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "widget" => toggle_window(app, "widget"),
                        "show" => toggle_window(app, "main"),
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .build(app)?;

                // Keep the floating widget above everything and on every workspace,
                // so it stays put when the desktop is shown / other apps minimize.
                if let Some(widget) = app.get_webview_window("widget") {
                    let _ = widget.set_always_on_top(true);
                    let _ = widget.set_visible_on_all_workspaces(true);
                    let _ = widget.set_skip_taskbar(true);
                }

                // Listen (from Rust, always running) to the daemon's wake channel and
                // summon the widget natively — no dependency on the hidden widget's
                // webview being alive. Emits `overseer-wake` so the widget starts to listen.
                {
                    use tauri::Emitter;
                    let handle = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        use futures_util::StreamExt;
                        use tokio_tungstenite::tungstenite::Message;
                        loop {
                            if let Ok((mut ws, _)) =
                                tokio_tungstenite::connect_async("ws://127.0.0.1:8080/api/ws/events").await
                            {
                                while let Some(Ok(msg)) = ws.next().await {
                                    if let Message::Text(t) = msg {
                                        if t.contains("wake") {
                                            if let Some(w) = handle.get_webview_window("widget") {
                                                let _ = w.show();
                                                let _ = w.set_focus();
                                                let _ = w.emit("overseer-wake", ());
                                            }
                                        }
                                    }
                                }
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                    });
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Close to tray instead of quitting — Overseer stays resident.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
