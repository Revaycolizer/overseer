use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::powershell::run_powershell_hidden;

pub async fn windows_process_list_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let script = r#"
$all = Get-Process
$top = $all |
    Sort-Object -Property CPU -Descending |
    Select-Object -First 20 ProcessName, Id, CPU, WS, MainWindowTitle
$background = $all |
    Where-Object { -not $_.MainWindowTitle -or $_.MainWindowTitle.Trim().Length -eq 0 } |
    Sort-Object -Property CPU -Descending |
    Select-Object -First 20 ProcessName, Id, CPU, WS
[pscustomobject]@{
    action = 'list_processes'
    total_processes = @($all).Count
    background_count = @($background).Count
    top_processes = $top
    background_processes = $background
} | ConvertTo-Json -Depth 5 -Compress
"#;

    run_powershell_hidden(script, tool_name).await
}
