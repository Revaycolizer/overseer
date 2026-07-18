use serde_json::json;

pub fn stringify_tool_input(args: &serde_json::Value) -> String {
    if let Some(input) = args.get("input") {
        if let Some(text) = input.as_str() {
            return text.to_string();
        }
        return input.to_string();
    }

    if let Some(query) = args.get("query").and_then(|value| value.as_str()) {
        return query.to_string();
    }

    if let Some(url) = args.get("url").and_then(|value| value.as_str()) {
        return url.to_string();
    }

    if let Some(name) = args.get("app").and_then(|value| value.as_str()) {
        return name.to_string();
    }

    args.to_string()
}

pub fn object_tool_input(args: &serde_json::Value) -> serde_json::Value {
    if let Some(input) = args.get("input") {
        if input.is_object() {
            return input.clone();
        }
        if let Some(raw) = input.as_str() {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
                return parsed;
            }
            return json!({ "input": raw });
        }
        return json!({ "input": input.to_string() });
    }

    args.clone()
}

pub fn read_string_field(input: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        input
            .get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    })
}

pub fn read_i64_field(input: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        input.get(*key).and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str()?.trim().parse().ok())
        })
    })
}
