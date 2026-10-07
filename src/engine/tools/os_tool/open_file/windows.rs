use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::fields::escape_powershell_single_quoted;
use crate::engine::tools::os_tool::powershell::run_powershell_hidden;

pub async fn windows_open_file_json(
    tool_name: &str,
    path: &str,
) -> Result<String, DynamicToolError> {
    let escaped_path = escape_powershell_single_quoted(path);
    let script = format!(
        r#"
$path = '{path}'
if (-not (Test-Path $path)) {{
    throw "File not found: $path"
}}
Start-Process -FilePath $path
[pscustomobject]@{{
    action = 'open_file'
    path = (Resolve-Path $path).Path
    ok = $true
}} | ConvertTo-Json -Compress
"#,
        path = escaped_path
    );

    run_powershell_hidden(&script, tool_name).await
}
