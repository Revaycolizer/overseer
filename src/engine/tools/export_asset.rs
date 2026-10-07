use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::object_tool_input;
use serde_json::json;
use std::path::PathBuf;

pub async fn native_export_asset(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let input = object_tool_input(args);
    let asset_path = input.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
        DynamicToolError::ExecutionFailed {
            tool: "export_asset".to_string(),
            details: "Missing 'path' to export.".to_string(),
        }
    })?;

    let raw_source = std::path::Path::new(asset_path);
    let source_path_buf = if raw_source.exists() {
        raw_source.to_path_buf()
    } else {
        let clean_path = asset_path.trim_start_matches('/');
        let rel_path = std::path::Path::new(clean_path);
        if rel_path.exists() {
            rel_path.to_path_buf()
        } else {
            let data_dir = crate::db::get_overseer_data_dir();
            let data_path = data_dir.join(clean_path);
            if data_path.exists() {
                data_path
            } else {
                return Err(DynamicToolError::ExecutionFailed {
                    tool: "export_asset".to_string(),
                    details: format!("Source asset not found: {}", asset_path),
                });
            }
        }
    };
    let source = &source_path_buf;

    let filename = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("exported_file");

    let download_dir = dirs::download_dir().unwrap_or_else(|| {
        #[cfg(not(windows))]
        {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()))
                .join("Downloads")
        }
        #[cfg(windows)]
        {
            PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string()))
                .join("Downloads")
        }
    });

    if !download_dir.exists() {
        let _ = std::fs::create_dir_all(&download_dir);
    }

    let destination = download_dir.join(filename);

    std::fs::copy(source, &destination).map_err(|e| DynamicToolError::ExecutionFailed {
        tool: "export_asset".to_string(),
        details: format!("Failed to copy to Downloads: {}", e),
    })?;

    Ok(json!({
        "status": "success",
        "message": format!("File exported to Downloads: {}", filename),
        "destination": destination.display().to_string()
    })
    .to_string())
}
