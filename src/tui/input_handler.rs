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
            app.input_cursor = idx + c.len_utf8();
            true
        }
        _ => false,
    }
}

fn backspace(app: &mut App) {
    if app.input_cursor > 0 {
        let idx = previous_boundary(&app.input, app.input_cursor);
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
        app.input_cursor = previous_boundary(&app.input, app.input_cursor);
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
        app.input_cursor = next_boundary(&app.input, app.input_cursor);
    }
}

fn previous_boundary(input: &str, cursor: usize) -> usize {
    input[..cursor].char_indices().next_back().map_or(0, |(idx, _)| idx)
}

fn next_boundary(input: &str, cursor: usize) -> usize {
    cursor + input[cursor..].chars().next().map_or(0, char::len_utf8)
}

#[cfg(test)]
mod tests {
    use super::{next_boundary, previous_boundary};

    #[test]
    fn cursor_crosses_multibyte_characters_without_splitting_them() {
        let input = "aé🦀界";
        let mut cursor = 0;
        for expected in [1, 3, 7, 10, 10] {
            cursor = next_boundary(input, cursor);
            assert_eq!(cursor, expected);
            assert!(input.is_char_boundary(cursor));
        }
        for expected in [7, 3, 1, 0, 0] {
            cursor = previous_boundary(input, cursor);
            assert_eq!(cursor, expected);
            assert!(input.is_char_boundary(cursor));
        }
    }

    #[test]
    fn backspace_removes_a_complete_character() {
        let mut input = String::from("hi🦀é");
        let cursor = previous_boundary(&input, input.len());
        input.remove(cursor);
        assert_eq!(input, "hi🦀");
        let cursor = previous_boundary(&input, cursor);
        input.remove(cursor);
        assert_eq!(input, "hi");
    }
}
