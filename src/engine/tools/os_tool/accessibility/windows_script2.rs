pub const SCRIPT_PART2: &str = r#"
function Matches-AppHint($element, $selector) {
    if ($null -eq $element -or -not $selector.app_hint) { return $false }
    $hint = [string]$selector.app_hint
    $name = (try { $element.Current.Name } catch { '' })
    $className = (try { $element.Current.ClassName } catch { '' })
    $processName = ''
    try {
        $processId = $element.Current.ProcessId
        if ($processId -gt 0) {
            $processName = (Get-Process -Id $processId -ErrorAction Stop).ProcessName
        }
    } catch {}
    return $name -like ('*' + $hint + '*') -or
        $className -like ('*' + $hint + '*') -or
        $processName -like ('*' + $hint + '*')
}
$preferredRoot = $null
$desktopRoot = [System.Windows.Automation.AutomationElement]::RootElement
if ($selector.app_hint -and $null -ne $desktopRoot) {
    try {
        $topWindows = $desktopRoot.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
        for ($i = 0; $i -lt $topWindows.Count; $i++) {
            $candidateWindow = $topWindows.Item($i)
            if (Matches-AppHint $candidateWindow $selector) {
                $preferredRoot = $candidateWindow
                break
            }
        }
    } catch {}
}
$focused = $null
try { $focused = [System.Windows.Automation.AutomationElement]::FocusedElement } catch {}
$searchRoots = New-Object System.Collections.Generic.List[System.Windows.Automation.AutomationElement]
if ($null -ne $preferredRoot) {
    $searchRoots.Add($preferredRoot)
}
if ($null -ne $root) {
    try {
        if ($null -eq $preferredRoot -or $preferredRoot.Current.NativeWindowHandle -ne $root.Current.NativeWindowHandle) {
            $searchRoots.Add($root)
        }
    } catch {
        $searchRoots.Add($root)
    }
}
if ($null -ne $desktopRoot) {
    $rootHandle = $null
    $desktopHandle = $null
    try { $rootHandle = $root.Current.NativeWindowHandle } catch {}
    try { $desktopHandle = $desktopRoot.Current.NativeWindowHandle } catch {}
    if ($rootHandle -ne $desktopHandle) {
        $searchRoots.Add($desktopRoot)
    }
}
$found = @()
$searchScope = 'active_window'
foreach ($searchRoot in $searchRoots) {
    try {
        $all = $searchRoot.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        $max = [Math]::Min($all.Count, 180)
        for ($i = 0; $i -lt $max; $i++) {
            $candidate = $all.Item($i)
            if (Matches-Selector $candidate $selector) {
                $found += Convert-UiElement($candidate)
            }
        }
        if (@($found).Count -gt 0) {
            try {
                if ($null -ne $preferredRoot -and $searchRoot.Current.NativeWindowHandle -eq $preferredRoot.Current.NativeWindowHandle) {
                    $searchScope = 'preferred_app_window'
                } elseif ($searchRoot.Current.NativeWindowHandle -eq $desktopRoot.Current.NativeWindowHandle) {
                    $searchScope = 'desktop_root'
                }
            } catch {}
            break
        }
    } catch {}
}
[pscustomobject]@{
    action = 'inspect_accessibility'
    selector = $selector
    focused_element = Convert-UiElement($focused)
    search_scope = $searchScope
    matches = $found | Select-Object -First 20
    match_count = @($found).Count
} | ConvertTo-Json -Depth 6 -Compress
"#;
