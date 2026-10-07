mod backend;
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
    let mut builder = tauri::Builder::default()
        .manage(backend::BackendState(std::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![backend::desktop_connection]);

    // Single-instance MUST be registered first: a second launch (e.g. autostart +
    // a manual run) just summons the existing window instead of starting again and
    // colliding on the global hotkey.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            for label in ["main", "widget"] {
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
        use tauri_plugin_global_shortcut::ShortcutState;
        builder = builder
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ))
            .plugin(
                tauri_plugin_global_shortcut::Builder::new()
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
                use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
                let toggle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
                if let Err(error) = app.global_shortcut().register(toggle) {
                    eprintln!(
                        "Global shortcut unavailable: {error}. Use the tray menu to open Overseer."
                    );
                }
                let widget =
                    MenuItem::with_id(app, "widget", "Assistant Widget", true, None::<&str>)?;
                let show = MenuItem::with_id(app, "show", "Open Full App", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&widget, &show, &quit])?;
                let _tray = TrayIconBuilder::new()
                    .title("Overseer")
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
                        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
                        use tokio_tungstenite::tungstenite::Message;
                        loop {
                            let connection = handle.state::<backend::BackendState>().0.lock().ok()
                                .and_then(|state| state.as_ref().map(|b| b.connection.clone()));
                            let Some(connection) = connection else {
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                continue;
                            };
                            let mut request = connection.url.replace("/api/ws/chat", "/api/ws/events")
                                .into_client_request()
                                .expect("static WebSocket URL");
                            {
                                let token = connection.token;
                                if let Ok(value) = format!("Bearer {token}").parse() {
                                    request.headers_mut().insert("authorization", value);
                                }
                            }
                            if let Ok((mut ws, _)) = tokio_tungstenite::connect_async(request).await
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
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Ok(mut state) = app.state::<backend::BackendState>().0.lock() {
                    *state = None;
                }
            }
        });
}
