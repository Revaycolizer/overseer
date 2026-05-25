use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::powershell::run_powershell_hidden;
use crate::engine::tools::os_tool::fields::escape_powershell_single_quoted;

pub async fn windows_read_file_json(tool_name: &str, path: &str) -> Result<String, DynamicToolError> {
    let escaped_path = escape_powershell_single_quoted(path);
    let script = format!(
        r#"
$path = '{path}'
if (-not (Test-Path $path)) {{
    throw "File not found: $path"
}}
$item = Get-Item $path
$content = $null
if ($item.Extension -in @('.txt','.md','.json','.rs','.toml','.yaml','.yml','.log','.csv')) {{
    $content = Get-Content -Path $path -Raw -ErrorAction SilentlyContinue
    if ($content -and $content.Length -gt 4000) {{
        $content = $content.Substring(0, 4000)
    }}
}}
[pscustomobject]@{{
    action = 'read_file'
    path = $item.FullName
    name = $item.Name
    extension = $item.Extension
    size = $item.Length
    preview = $content
}} | ConvertTo-Json -Depth 4 -Compress
"#,
        path = escaped_path
    );

    run_powershell_hidden(&script, tool_name).await
}
