use crate::engine::tools::os_tool::workspace_search::scoring::{
    score_file_match, tokenize_file_query,
};
use serde_json::json;
use std::path::PathBuf;
use tokio::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub async fn search_workspace_files_fast(query: &str) -> Option<Vec<serde_json::Value>> {
    let cwd = std::env::current_dir().ok()?;
    let query_lower = query.trim().to_lowercase();
    if query_lower.is_empty() {
        return None;
    }

    let tokens = tokenize_file_query(query);
    let mut command = Command::new("rg");
    command.arg("--files").arg(&cwd);
    command.kill_on_drop(true);

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output().await.ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut matches: Vec<(i32, PathBuf)> = stdout
        .lines()
        .filter_map(|line| {
            let relative = line.trim();
            if relative.is_empty() {
                return None;
            }

            let absolute = cwd.join(relative);
            let path_lower = absolute.to_string_lossy().to_lowercase();
            let name_lower = absolute
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_lowercase();

            let score = score_file_match(&path_lower, &name_lower, &query_lower, &tokens);
            if score <= 0 {
                return None;
            }

            Some((score, absolute))
        })
        .collect();

    matches.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.to_string_lossy().cmp(&right.1.to_string_lossy()))
    });

    matches.dedup_by(|left, right| left.1 == right.1);

    let results = matches
        .into_iter()
        .take(20)
        .map(|(_, path)| {
            json!({
                "name": path.file_name().and_then(|value| value.to_str()).unwrap_or_default(),
                "path": path.to_string_lossy().to_string(),
                "directory": path.parent().map(|value| value.to_string_lossy().to_string()).unwrap_or_default(),
                "extension": path.extension().map(|value| format!(".{}", value.to_string_lossy())).unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}
