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
        "browser_navigate" => Some(crate::engine::tools::browser::navigate(args).await?),
        "browser_screenshot" => Some(crate::engine::tools::browser::screenshot(args).await?),
        "app_launcher" => Some(crate::engine::tools::app_launcher::native_app_launcher(args).await?),
        "document_writer" => {
            tracing::debug!("Calling native_document_writer");
            match crate::engine::tools::document_generation::native_document_writer(args).await {
                Ok(r) => {
                    tracing::debug!("native_document_writer returned");
                    Some(r)
                },
                Err(e) => {
                    tracing::error!(?e, "native_document_writer failed");
                    return Err(DynamicToolError::ExecutionFailed {
                        tool: "document_writer".to_string(),
                        details: e,
                    });
                }
            }
        },
        "export_asset" => Some(crate::engine::tools::export_asset::native_export_asset(args).await?),
        "os_tool" => Some(crate::engine::tools::os_tool::native_os_tool(args).await?),
        "vision_analysis" => Some(crate::engine::tools::vision::native_vision_analysis(args).await?),
        "charting" => {
            let input_str = if let Some(input) = args.get("input") {
                if input.is_string() { input.as_str().unwrap_or("").to_string() } else { input.to_string() }
            } else { args.to_string() }.trim().to_string();
            let req: crate::engine::tools::charting::ChartRequest = serde_json::from_str(&input_str)
                .or_else(|_| { let val: serde_json::Value = serde_json::from_str(&input_str)?; serde_json::from_value(val) })
                .map_err(|e| DynamicToolError::ExecutionFailed { tool: "charting".to_string(), details: e.to_string() })?;
            match crate::engine::tools::charting::generate_chart_svg(req.clone()) {
                Ok(_) => Some(serde_json::to_string(&req).unwrap_or(input_str)),
                Err(e) => return Err(DynamicToolError::ExecutionFailed { tool: "charting".to_string(), details: e }),
            }
        },
        _ => None,
    };

    Ok(output)
}
