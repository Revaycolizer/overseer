use std::sync::Arc;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use overseer::db::Database;
use overseer::backend::client::BackendClient;
use overseer::tui::app::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::sink)
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("overseer-tui {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Overseer terminal {}\n\nUsage: overseer-tui [--remote] [--url URL]\n\nRuns standalone by default. --url requires --remote.\nF1: help; Ctrl+O: installed Ollama models. Configure providers in Settings.\n--version: print version; --help: print this help.", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    for (idx, arg) in args.iter().enumerate().skip(1) {
        if args.get(idx.wrapping_sub(1)).map(String::as_str) == Some("--url") {
            continue;
        }
        if !matches!(arg.as_str(), "--remote" | "remote" | "--url") {
            return Err(format!("Unknown argument: {arg}. Use --help.").into());
        }
    }
    if let Some(idx) = args.iter().position(|arg| arg == "--url") {
        let value = args.get(idx + 1).filter(|value| !value.starts_with('-'))
            .ok_or("--url requires a URL")?;
        if !args.iter().any(|arg| arg == "--remote" || arg == "remote") {
            return Err("--url requires --remote".into());
        }
        let url = reqwest::Url::parse(value)?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err("--url must be an HTTP or HTTPS URL".into());
        }
    }
    let is_remote = args.iter().any(|arg| arg == "--remote" || arg == "remote");
    let remote_url = args.iter().position(|arg| arg == "--url").and_then(|idx| args.get(idx + 1)).map(|s| s.as_str()).unwrap_or("http://127.0.0.1:8080");

    let db = if is_remote {
        Database::new_remote(remote_url).await
    } else {
        Database::new().await?
    };

    let db = Arc::new(db);
    let agent_system = Arc::new(overseer::engine::system::AgentSystem::new((*db).clone()));
    let client: Arc<dyn BackendClient + Send + Sync> = if db.is_remote() {
        Arc::new(db.remote_client())
    } else {
        Arc::new(overseer::backend::LocalClient::new(db.clone(), agent_system.clone()))
    };

    // Initialization may fail (for example, when the remote daemon is offline).
    // Keep the terminal in its normal mode until that work has succeeded.
    let mut app = App::new(client, db).await?;

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        tracing::error!(?err, "TUI run loop exited with error");
    }
    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>>
where
    B::Error: 'static,
{
    let mut last_refresh = std::time::Instant::now();
    let mut last_seen_activity = app.activity.current();
    let mut last_seen_engine = overseer::engine::activity::engine_activity().current();
    let mut redraw = true;
    loop {
        if redraw {
            terminal.draw(|f| overseer::tui::ui::draw(f, app))?;
            redraw = false;
        }

        if app.is_orchestrating.load(std::sync::atomic::Ordering::SeqCst) {
            if last_refresh.elapsed() >= std::time::Duration::from_secs(1) {
                let _ = app.refresh_conversations().await;
                redraw = true;
                last_refresh = std::time::Instant::now();
            }
        } else if app.status_message == "Orchestrating agent run..." {
            let _ = app.refresh_conversations().await;
            redraw = true;
            if let Some(err) = app.orchestrate_error.lock().unwrap().take() {
                app.status_message = format!("Error: {}", err);
            } else {
                app.status_message = "Ready.".to_string();
            }
            last_seen_activity = app.activity.current();
            last_seen_engine = overseer::engine::activity::engine_activity().current();
            last_refresh = std::time::Instant::now();
        } else if app.activity.changed_since(last_seen_activity)
            || overseer::engine::activity::engine_activity().changed_since(last_seen_engine)
        {
            // A conversation-mutating call (send_message, add_message,
            // add_conversation) or an engine-side write (LLM-refined
            // title, future background DB writes) bumped a counter.
            // Refresh now instead of waiting for a fixed interval.
            let _ = app.refresh_conversations().await;
            redraw = true;
            last_seen_activity = app.activity.current();
            last_seen_engine = overseer::engine::activity::engine_activity().current();
            last_refresh = std::time::Instant::now();
        } else {
            // Post-orchestrate poll window: after `is_orchestrating` flips
            // back to false, the LLM-refined title lands in the DB
            // 1–3 s later via a detached `tokio::spawn` inside
            // `agent_system::orchestrate`. Poll every 1 s for 5 s after
            // the spawn returns so the refined title surfaces without
            // waiting on a counter change.
            let in_post_window = app
                .last_orchestrate_finished
                .lock()
                .unwrap()
                .map(|t| t.elapsed() < std::time::Duration::from_secs(5))
                .unwrap_or(false);
            if in_post_window && last_refresh.elapsed() >= std::time::Duration::from_secs(1) {
                let _ = app.refresh_conversations().await;
                redraw = true;
                last_refresh = std::time::Instant::now();
            } else if !in_post_window
                && last_refresh.elapsed() >= std::time::Duration::from_secs(30)
            {
                // Long safety poll in case a counter increment was
                // missed (e.g. a remote mutation that didn't go through
                // the TUI's send path).
                let _ = app.refresh_conversations().await;
                redraw = true;
                last_refresh = std::time::Instant::now();
            }
        }

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == event::KeyEventKind::Press => {
                    if (key.code == KeyCode::Char('q') || key.code == KeyCode::Char('c'))
                        && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        app.cancel_current_orchestrate();
                        return Ok(());
                    }
                    if let Err(e) = overseer::tui::handlers::handle_key(app, key).await {
                        app.status_message = format!("Error: {e}");
                    }
                    redraw = true;
                }
                Event::Resize(_, _) => redraw = true,
                _ => {}
            }
        }
    }
}
