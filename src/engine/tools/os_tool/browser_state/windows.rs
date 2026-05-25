use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::powershell::run_powershell_hidden;

pub async fn windows_browser_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let script = r#"
$browserNames = @('chrome','msedge','firefox','brave','opera')
$windows = @(Get-Process |
    Where-Object { $browserNames -contains $_.ProcessName.ToLower() -and $_.MainWindowTitle -and $_.MainWindowTitle.Trim().Length -gt 0 } |
    Select-Object -First 12 ProcessName, Id, MainWindowTitle)
[pscustomobject]@{
    action = 'browser_state'
    browser_windows = $windows
    browser_count = @($windows).Count
} | ConvertTo-Json -Depth 4 -Compress
"#;

    run_powershell_hidden(script, tool_name).await
}
