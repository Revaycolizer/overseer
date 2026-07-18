use serde_json::json;
use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::workspace_search::search_workspace_files_fast;
use crate::engine::tools::os_tool::find_files::unix_search::{get_unix_search_roots, execute_unix_search};
use crate::engine::tools::os_tool::find_files::unix_scoring::process_unix_search_results;

pub async fn windows_find_files_json(tool_name: &str, query: &str) -> Result<String, DynamicToolError> {
    if let Some(results) = search_workspace_files_fast(query).await {
        let cwd = std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().to_string())
            .into_iter()
            .collect::<Vec<_>>();

        return Ok(json!({
            "action": "find_files",
            "query": query,
            "roots": cwd,
            "source": "workspace_fast_path",
            "results": results,
        })
        .to_string());
    }

    let roots = get_unix_search_roots();
    let file_lines = execute_unix_search(&roots, tool_name).await?;
    let results = process_unix_search_results(file_lines, query);

    Ok(json!({
        "action": "find_files",
        "query": query,
        "roots": roots.into_iter().map(|path| path.to_string_lossy().to_string()).collect::<Vec<_>>(),
        "source": "unix_native_search",
        "results": results,
    })
    .to_string())
}
