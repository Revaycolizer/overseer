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
    let args: Vec<String> = std::env::args().collect();
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

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(client).await?;
    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }
    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut last_refresh = std::time::Instant::now();
    loop {
        terminal.draw(|f| overseer::tui::ui::draw(f, app))?;

        if app.is_orchestrating.load(std::sync::atomic::Ordering::SeqCst) {
            if last_refresh.elapsed() >= std::time::Duration::from_secs(1) {
                let _ = app.refresh_conversations().await;
                last_refresh = std::time::Instant::now();
            }
        } else if app.status_message == "Orchestrating agent run..." {
            let _ = app.refresh_conversations().await;
            if let Some(err) = app.orchestrate_error.lock().unwrap().take() {
                app.status_message = format!("Error: {}", err);
            } else {
                app.status_message = "Ready.".to_string();
            }
        }

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    if key.code == KeyCode::Char('q') {
                        return Ok(());
                    }
                    overseer::tui::handlers::handle_key(app, key).await?;
                }
            }
        }
    }
}
