use enigo::Key;
use crate::engine::tools::input::read_string_field;

pub fn extract_key_tokens(input: &serde_json::Value) -> Vec<String> {
    if let Some(keys) = input.get("keys").and_then(|value| value.as_array()) {
        let tokens: Vec<String> = keys
            .iter()
            .filter_map(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if !tokens.is_empty() {
            return tokens;
        }
    }

    if let Some(key) = read_string_field(input, &["key", "shortcut", "combo"]) {
        return key
            .split(|c| matches!(c, '+' | ',' | ' '))
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(ToString::to_string)
            .collect();
    }

    if let Some(raw) = read_string_field(input, &["input"]) {
        let lower = raw.to_lowercase();
        let prefixes = ["press ", "hotkey ", "shortcut ", "key combo ", "combo "];
        for prefix in prefixes {
            if lower.starts_with(prefix) {
                return raw[prefix.len()..]
                    .split(|c| matches!(c, '+' | ',' | ' '))
                    .map(str::trim)
                    .filter(|token| !token.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
        }
    }

    Vec::new()
}

pub fn key_from_token(token: &str) -> Option<Key> {
    let lower = token.trim().to_lowercase();
    Some(match lower.as_str() {
        "ctrl" | "control" => Key::Control,
        "alt" | "option" => Key::Alt,
        "shift" => Key::Shift,
        "meta" | "win" | "windows" | "super" | "cmd" | "command" => Key::Meta,
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "esc" | "escape" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "space" => Key::Space,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "pageup" | "page_up" => Key::PageUp,
        "pagedown" | "page_down" => Key::PageDown,
        "home" => Key::Home,
        "end" => Key::End,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        _ => {
            let mut chars = lower.chars();
            let first = chars.next()?;
            if chars.next().is_none() {
                Key::Unicode(first)
            } else {
                return None;
            }
        }
    })
}
