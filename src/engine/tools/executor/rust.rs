use crate::engine::tools::error::DynamicToolError;
use crate::engine::utils::discovery::ToolInfo;

pub async fn execute_rust_tool(
    info: &ToolInfo,
    args: serde_json::Value,
) -> Result<String, DynamicToolError> {
    match info.name.as_str() {
        "os_tool" => crate::engine::tools::os_tool::native_os_tool(&args).await,
        "vision_analysis" => crate::engine::tools::vision::native_vision_analysis(&args).await,
        "charting" => {
            let input_str = if let Some(input) = args.get("input") {
                if input.is_string() {
                    input.as_str().unwrap_or("").to_string()
                } else {
                    input.to_string()
                }
            } else {
                args.to_string()
            }
            .trim()
            .to_string();

            tracing::debug!("CHARTING INPUT received");

            let req: crate::engine::tools::charting::ChartRequest = serde_json::from_str(&input_str)
                .or_else(|_| {
                    let val: serde_json::Value = serde_json::from_str(&input_str)?;
                    serde_json::from_value(val)
                })
                .map_err(|e| {
                    DynamicToolError::InvalidChartRequest(format!(
                        "{}. Provide {{\"chart_type\":\"bar|line|pie\",\"title\":\"...\",\"labels\":[...],\"data\":[...]}}",
                        e
                    ))
                })?;

            crate::engine::tools::charting::generate_chart_svg(req.clone()).map_err(|details| {
                DynamicToolError::ExecutionFailed {
                    tool: info.name.clone(),
                    details,
                }
            })?;

            Ok(serde_json::to_string(&req).unwrap_or(input_str))
        }
        _ => Err(DynamicToolError::NativeNotImplemented(info.name.clone())),
    }
}
