use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::fields::escape_powershell_single_quoted;
use crate::engine::tools::os_tool::powershell::run_powershell_hidden;
use crate::engine::tools::os_tool::workspace_search::search_workspace_files_fast;
use serde_json::json;

pub async fn windows_find_files_json(
    tool_name: &str,
    query: &str,
) -> Result<String, DynamicToolError> {
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

    let escaped_query = escape_powershell_single_quoted(query);
    let script = format!(
        r#"
$query = '{query}'
$home = [Environment]::GetFolderPath('UserProfile')
$cwd = [System.IO.Directory]::GetCurrentDirectory()
$roots = @(
    $cwd,
    (Join-Path $home 'Desktop'),
    (Join-Path $home 'Documents'),
    (Join-Path $home 'Downloads'),
    (Join-Path $home 'OneDrive')
) | Where-Object {{ $_ -and (Test-Path $_) }} | Select-Object -Unique

$tokens = $query.ToLower().Split(@(' ', '?', '.', ',', ':', ';', '\', '/'), [System.StringSplitOptions]::RemoveEmptyEntries) |
    Where-Object {{ $_.Length -ge 3 -and $_ -notin @('find','path','file','files','pc','computer','my','the','in') }}
if (-not $tokens -or $tokens.Count -eq 0) {{
    $tokens = @($query.ToLower())
}}

$seen = New-Object 'System.Collections.Generic.HashSet[string]'
$matches = New-Object System.Collections.Generic.List[object]
foreach ($root in $roots) {{
    try {{
        foreach ($token in $tokens) {{
            Get-ChildItem -Path $root -Recurse -File -Filter "*$token*" -ErrorAction SilentlyContinue |
                Select-Object -First 25 |
                ForEach-Object {{
                $normalized = $_.FullName.ToLower()
                if (-not $seen.Add($normalized)) {{
                    return
                }}
                $full = $_.FullName.ToLower()
                $name = $_.Name.ToLower()
                $score = 0
                if ($name -eq $query.ToLower()) {{ $score += 200 }}
                foreach ($token in $tokens) {{
                    if ($name -eq $token) {{ $score += 80 }}
                    if ($name -like "*$token*") {{ $score += 30 }}
                    if ($full -like "*$token*") {{ $score += 10 }}
                }}
                if ($full -like "*\overseer\src\*") {{ $score += 40 }}
                if ($full -like "*\desktop\*") {{ $score += 5 }}

                $matches.Add([pscustomobject]@{{
                    name = $_.Name
                    path = $_.FullName
                    directory = $_.DirectoryName
                    extension = $_.Extension
                    score = $score
                }})
                }}

            if ($matches.Count -ge 40) {{ break }}
        }}
    }} catch {{}}
    if ($matches.Count -ge 40) {{ break }}
}}

if ($matches.Count -eq 0) {{
    foreach ($root in $roots) {{
        try {{
            Get-ChildItem -Path $root -Recurse -File -ErrorAction SilentlyContinue |
                Where-Object {{
                    $name = $_.Name.ToLower()
                    $full = $_.FullName.ToLower()
                    foreach ($token in $tokens) {{
                        if ($name -like "*$token*" -or $full -like "*$token*") {{ return $true }}
                    }}
                    return $false
                }} |
                Select-Object -First 20 |
                ForEach-Object {{
                    $normalized = $_.FullName.ToLower()
                    if (-not $seen.Add($normalized)) {{
                        return
                    }}
                    $matches.Add([pscustomobject]@{{
                        name = $_.Name
                        path = $_.FullName
                        directory = $_.DirectoryName
                        extension = $_.Extension
                        score = 1
                    }})
                }}
        }} catch {{}}
        if ($matches.Count -ge 20) {{ break }}
    }}
}}

$results = $matches |
    Sort-Object -Property @{{Expression='score';Descending=$true}}, @{{Expression='path';Descending=$false}} |
    Select-Object -First 20 name, path, directory, extension

[pscustomobject]@{{
    action = 'find_files'
    query = $query
    roots = $roots
    results = $results
}} | ConvertTo-Json -Depth 5 -Compress
"#,
        query = escaped_query
    );

    run_powershell_hidden(&script, tool_name).await
}
