//! Typed filesystem-write tool.
//!
//! Writes `content` to `path` using `std::fs` — never a shell (`sh -c`, `echo`,
//! PowerShell, ...). This is a side-effecting capability, so at the dispatch
//! chokepoint it is classified as `ActionKind::FilesystemWrite` and governed by
//! the kernel (approval-required by default); this function performs the write
//! only after the kernel has authorized it.

use crate::engine::tools::error::DynamicToolError;
use serde_json::{json, Value};

fn fail(details: impl Into<String>) -> DynamicToolError {
    DynamicToolError::ExecutionFailed {
        tool: "write_file".to_string(),
        details: details.into(),
    }
}

/// Execute a `write_file` action. Accepts `{ "path": ..., "content": ... }`
/// either at the top level or nested under `"input"`.
pub async fn native_write_file(args: &Value) -> Result<String, DynamicToolError> {
    let nested = args.get("input").filter(|v| v.is_object());
    let lookup = |key: &str| -> Option<String> {
        nested
            .and_then(|n| n.get(key))
            .or_else(|| args.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    let path = lookup("path")
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .ok_or_else(|| fail("missing required 'path'"))?;
    let content = lookup("content").unwrap_or_default();

    let p = std::path::Path::new(&path);

    // Create parent directories as needed.
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| fail(format!("failed to create directory {}: {}", parent.display(), e)))?;
        }
    }

    std::fs::write(p, content.as_bytes())
        .map_err(|e| fail(format!("failed to write {}: {}", path, e)))?;

    let bytes = content.len();
    Ok(json!({
        "status": "completed",
        "tool": "write_file",
        "path": path,
        "bytes_written": bytes,
        "summary": format!("Wrote {} byte(s) to {}", bytes, path),
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writes_file_with_content_creating_parents() {
        let dir = std::env::temp_dir().join(format!("overseer-wf-test-{}", std::process::id()));
        let path = dir.join("nested").join("proof.txt");
        let _ = std::fs::remove_dir_all(&dir);

        let args = json!({ "path": path.to_string_lossy(), "content": "KERNEL_OK" });
        let out = native_write_file(&args).await.expect("write should succeed");

        assert!(path.exists(), "file must exist after write");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "KERNEL_OK");
        assert!(out.contains("bytes_written"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn missing_path_is_an_error() {
        let args = json!({ "content": "x" });
        assert!(native_write_file(&args).await.is_err());
    }

    #[tokio::test]
    async fn accepts_nested_input_object() {
        let dir = std::env::temp_dir().join(format!("overseer-wf-nested-{}", std::process::id()));
        let path = dir.join("a.txt");
        let _ = std::fs::remove_dir_all(&dir);
        let args = json!({ "input": { "path": path.to_string_lossy(), "content": "hi" } });
        native_write_file(&args).await.expect("nested input should work");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hi");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
