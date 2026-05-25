use crate::engine::tools::error::DynamicToolError;
use crate::engine::utils::discovery::ToolInfo;

pub async fn call_native_builtin_tool(
    info: &ToolInfo,
    args: &serde_json::Value,
) -> Result<Option<String>, DynamicToolError> {
    let output = match info.name.as_str() {
        "clipboard_read" => Some(crate::engine::tools::clipboard::native_clipboard_read().await?),
        "clipboard_write" => Some(crate::engine::tools::clipboard::native_clipboard_write(args).await?),
        "http_request" => Some(crate::engine::tools::http::native_http_request(args).await?),
        "web_extract" => Some(crate::engine::tools::web_extract::native_web_extract(args).await?),
        "web_links" => Some(crate::engine::tools::web_links::native_web_links(args).await?),
        "web_browse" => Some(crate::engine::tools::web_browse::native_stealth_browse(args).await?),
        "stealth_browse" => Some(crate::engine::tools::web_browse::native_stealth_browse(args).await?),
        "web_search" => Some(crate::engine::tools::web_search::native_stealth_search(args).await?),
        "stealth_search" => Some(crate::engine::tools::web_search::native_stealth_search(args).await?),
        "app_launcher" => Some(crate::engine::tools::app_launcher::native_app_launcher(args).await?),
        "document_writer" => {
            println!("🛠️ DEBUG: Calling native_document_writer");
            match crate::engine::tools::document_generation::native_document_writer(args).await {
                Ok(r) => {
                    println!("🛠️ DEBUG: native_document_writer returned: {}", r);
                    Some(r)
                },
                Err(e) => {
                    println!("❌ DEBUG: native_document_writer failed: {}", e);
                    return Err(DynamicToolError::ExecutionFailed {
                        tool: "document_writer".to_string(),
                        details: e,
                    });
                }
            }
        },
        "export_asset" => Some(crate::engine::tools::export_asset::native_export_asset(args).await?),
        _ => None,
    };

    Ok(output)
}
