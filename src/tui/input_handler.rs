use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::app::App;

pub fn app(key: &KeyEvent, app: &mut App) -> bool {
    match key.code {
        KeyCode::Backspace => { backspace(app); true }
        KeyCode::Delete => { delete(app, key.modifiers); true }
        KeyCode::Left => { left(app, key.modifiers); true }
        KeyCode::Right => { right(app, key.modifiers); true }
        KeyCode::Home => { app.input_cursor = 0; true }
        KeyCode::End => { app.input_cursor = app.input.len(); true }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let idx = app.input_cursor.min(app.input.len());
            app.input.insert(idx, c);
            app.input_cursor = idx + 1;
            true
        }
        _ => false,
    }
}

fn backspace(app: &mut App) {
    if app.input_cursor > 0 {
        let idx = app.input_cursor - 1;
        app.input.remove(idx);
        app.input_cursor = idx;
    }
}

fn delete(app: &mut App, mods: KeyModifiers) {
    if mods.contains(KeyModifiers::CONTROL) {
        app.input.clear();
        app.input_cursor = 0;
    } else if app.input_cursor < app.input.len() {
        app.input.remove(app.input_cursor);
    }
}

fn left(app: &mut App, mods: KeyModifiers) {
    if mods.contains(KeyModifiers::CONTROL) {
        if let Some(prev_space) = app.input[..app.input_cursor].rfind(|c: char| c == ' ') {
            app.input_cursor = prev_space + 1;
        } else {
            app.input_cursor = 0;
        }
    } else {
        app.input_cursor = app.input_cursor.saturating_sub(1);
    }
}

fn right(app: &mut App, mods: KeyModifiers) {
    if mods.contains(KeyModifiers::CONTROL) {
        if let Some(next_space) = app.input[app.input_cursor..].find(' ') {
            app.input_cursor += next_space + 1;
        } else {
            app.input_cursor = app.input.len();
        }
    } else {
        app.input_cursor = (app.input_cursor + 1).min(app.input.len());
    }
}
