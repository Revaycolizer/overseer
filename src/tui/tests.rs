use super::{app::App, handlers::handle_key, tab::ActiveTab};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Fixture {
    has_chat: Arc<std::sync::atomic::AtomicBool>,
    settings: Arc<Mutex<HashMap<String, String>>>,
    messages: Arc<Mutex<Vec<serde_json::Value>>>,
}

async fn fixture() -> (App, Fixture, tokio::task::JoinHandle<()>) {
    let state = Fixture::default();
    let router = Router::new()
        .route("/api/conversations", get(|State(s): State<Fixture>| async move {
            Json(if s.has_chat.load(std::sync::atomic::Ordering::SeqCst) {
                vec![serde_json::json!({"id":42,"title":"New Conversation","created_at":"2026-10-04T00:00:00"})]
            } else { vec![] })
        }).post(|State(s): State<Fixture>| async move { s.has_chat.store(true, std::sync::atomic::Ordering::SeqCst); Json(42i64) }))
        .route("/api/messages", get(|| async { Json(Vec::<serde_json::Value>::new()) }).post(|State(s): State<Fixture>, Json(v): Json<serde_json::Value>| async move {
            s.messages.lock().unwrap().push(v); Json(1i64)
        }))
        .route("/api/settings/all", get(|State(s): State<Fixture>| async move { Json(s.settings.lock().unwrap().clone()) }))
        .route("/api/settings", post(|State(s): State<Fixture>, Json(v): Json<serde_json::Value>| async move {
            s.settings.lock().unwrap().insert(v["key"].as_str().unwrap().into(), v["value"].as_str().unwrap().into());
        }))
        .route("/api/models/available", get(|| async { Json(vec!["qwen3:14b", "llama3:latest"]) }))
        .route("/api/orchestrate", post(|| async { "test response" }))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let db = Arc::new(crate::db::Database::new_remote(&url).await);
    let client = Arc::new(crate::backend::RemoteClient::new(url));
    (App::new(client, db).await.unwrap(), state, server)
}

#[tokio::test]
async fn model_picker_lists_real_models_and_saves_chat_and_planner() {
    let (mut app, state, server) = fixture().await;
    handle_key(&mut app, KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE))
        .await
        .unwrap();
    assert_eq!(app.active_tab, ActiveTab::Settings);
    assert_eq!(app.available_models, ["llama3:latest", "qwen3:14b"]);
    handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .await
        .unwrap();
    handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .await
        .unwrap();
    assert_eq!(app.active_tab, ActiveTab::Chat);
    let settings = state.settings.lock().unwrap();
    assert_eq!(settings["default_chat_model"], "qwen3:14b");
    assert_eq!(settings["planner_model"], "qwen3:14b");
    assert_eq!(settings["llm_provider"], "ollama");
    drop(settings);
    server.abort();
}

#[tokio::test]
async fn first_send_creates_chat_resets_cursor_and_remains_editable() {
    let (mut app, state, server) = fixture().await;
    app.input = "Hello 🦀".into();
    app.input_cursor = app.input.len();
    app.send_message().await.unwrap();
    assert_eq!(app.selected_conv_id, Some(42));
    assert_eq!(app.input_cursor, 0);
    assert!(app.input.is_empty());
    assert_eq!(state.messages.lock().unwrap()[0]["content"], "Hello 🦀");
    app.cancel_current_orchestrate();
    app.is_orchestrating
        .store(false, std::sync::atomic::Ordering::SeqCst);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('é'), KeyModifiers::NONE),
    )
    .await
    .unwrap();
    assert_eq!(app.input, "é");
    server.abort();
}

#[tokio::test]
async fn narrow_terminal_renders_and_preserves_model_navigation() {
    let (mut app, _, server) = fixture().await;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 16)).unwrap();
    terminal.draw(|f| super::ui::draw(f, &mut app)).unwrap();
    handle_key(&mut app, KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE))
        .await
        .unwrap();
    terminal.draw(|f| super::ui::draw(f, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(text.contains("Installed models"));
    assert!(text.contains("llama3:latest"));
    server.abort();
}
