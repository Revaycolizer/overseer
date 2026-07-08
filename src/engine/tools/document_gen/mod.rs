pub mod pdf;
pub mod utils;
pub mod csv_gen;
pub mod xlsx_gen;
pub mod docx_gen;

use serde_json::{json, Value};
use utils::strip_grounding_markers;

pub async fn native_document_writer(args: &Value) -> Result<String, String> {
    let format = args

        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("pdf")
        .to_lowercase();

    let headers: Vec<String> = args
        .get("headers")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|i| i.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let data = args.get("data").and_then(|v| v.as_array());

    let content_str = build_content_string(args.get("content"));

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let data_dir = crate::db::get_overseer_data_dir();
    let export_dir = data_dir.join("assets").join("exports");
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;
    let prefix = format!("report_{}", timestamp);

    let res = match format.as_str() {
        "pdf" => pdf::generate_pdf(&export_dir, &prefix, None, None, Some(&content_str)),
        "csv" => csv_gen::generate_csv(&export_dir, &prefix, &headers, data.unwrap_or(&vec![])),
        "xlsx" => xlsx_gen::generate_xlsx(&export_dir, &prefix, &headers, data.unwrap_or(&vec![])),
        "docx" => docx_gen::generate_docx(&export_dir, &prefix, &content_str),
        _ => Err(format!("Unsupported format: {}", format)),
    };

    match res {
        Ok(file) => Ok(json!({
            "status": "success",
            "artifacts": [{"path": export_dir.join(&file).to_string_lossy().to_string(), "kind": format}]
        })
        .to_string()),
        Err(e) => Err(e),
    }
}

fn build_content_string(content: Option<&Value>) -> String {
    let c = match content { Some(v) => v, None => return String::new() };

    // Shape: {"title":"...", "sections":[{header,content}]}
    if let Some(sections) = c.get("sections").and_then(|v| v.as_array()) {
        let title = c.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let mut parts = Vec::new();
        if !title.is_empty() { parts.push(format!("# {}", title)); }
        for sec in sections {
            let h = sec.get("header").or_else(|| sec.get("section")).or_else(|| sec.get("title"))
                .and_then(|v| v.as_str()).unwrap_or("");
            let b = sec.get("content").or_else(|| sec.get("text")).and_then(|v| v.as_str()).unwrap_or("");
            if !h.is_empty() { parts.push(format!("## {}", h)); }
            if !b.is_empty() { parts.push(b.to_string()); }
        }
        return strip_grounding_markers(&parts.join("\n\n"));
    }

    // Shape: array of strings or {section/header, text/content} objects
    if let Some(arr) = c.as_array() {
        let joined = arr.iter().map(|item| {
            if let Some(s) = item.as_str() { return s.to_string(); }
            let h = item.get("section").or_else(|| item.get("header")).or_else(|| item.get("title"))
                .and_then(|v| v.as_str()).unwrap_or("");
            let b = item.get("text").or_else(|| item.get("content")).and_then(|v| v.as_str()).unwrap_or("");
            if h.is_empty() { b.to_string() } else { format!("## {}\n{}", h, b) }
        }).collect::<Vec<_>>().join("\n\n");
        return strip_grounding_markers(&joined);
    }

    strip_grounding_markers(c.as_str().unwrap_or(""))
}
