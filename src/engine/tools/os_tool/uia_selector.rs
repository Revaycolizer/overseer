use serde_json::json;
use crate::engine::tools::os_tool::uia_inference::{clean_uia_selector_name, infer_uia_app_hint_details, infer_uia_control_type, strip_trailing_app_hint};

pub fn has_uia_selector(input: &serde_json::Value) -> bool {
    input.get("selector").is_some()
        || input.get("element").is_some()
        || input.get("target").is_some()
}

pub fn extract_uia_selector_target(input: &serde_json::Value) -> String {
    input
        .get("selector")
        .or_else(|| input.get("element"))
        .or_else(|| input.get("target"))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub fn build_uia_selector_object(input: &serde_json::Value) -> serde_json::Value {
    if input.get("name").is_some() || input.get("control_type").is_some() || input.get("app_hint").is_some() {
        return input.clone();
    }
    let selector_str = extract_uia_selector_target(input);
    let app_details = infer_uia_app_hint_details(&selector_str);
    let name_without_app = strip_trailing_app_hint(&selector_str);
    let control_type = infer_uia_control_type(&name_without_app);
    let clean_name = clean_uia_selector_name(&name_without_app);

    let mut selector = json!({
        "name": clean_name
    });

    if let Some(ct) = control_type {
        selector["control_type"] = json!(ct);
    }

    if let Some((app_name, window_title)) = app_details {
        selector["app_hint"] = json!(app_name);
        selector["window_hint"] = json!(window_title);
    }

    selector
}
