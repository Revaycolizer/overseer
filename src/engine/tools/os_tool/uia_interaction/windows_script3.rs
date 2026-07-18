pub const SCRIPT_PART3: &str = r#"
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
$match = $null
$searchScope = 'active_window'
foreach ($searchRoot in $searchRoots) {
    try {
        $all = $searchRoot.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        $max = [Math]::Min($all.Count, 220)
        for ($i = 0; $i -lt $max; $i++) {
            $candidate = $all.Item($i)
            if (Matches-Selector $candidate $selector) {
                $match = $candidate
                try {
                    if ($null -ne $preferredRoot -and $searchRoot.Current.NativeWindowHandle -eq $preferredRoot.Current.NativeWindowHandle) {
                        $searchScope = 'preferred_app_window'
                    } elseif ($searchRoot.Current.NativeWindowHandle -eq $desktopRoot.Current.NativeWindowHandle) {
                        $searchScope = 'desktop_root'
                    }
                } catch {}
                break
            }
        }
    } catch {}
    if ($null -ne $match) {
        break
    }
}
if ($null -eq $match) {
    $focused = $null
    try { $focused = [System.Windows.Automation.AutomationElement]::FocusedElement } catch {}
    $activeWindow = Convert-UiElement($root)
    $candidateSearchRoot = if ($null -ne $preferredRoot) { $preferredRoot } elseif ($null -ne $root) { $root } else { $desktopRoot }
    $blocked = [pscustomobject]@{
        action = $action
        selector = $selector
        app_hint = $selector.app_hint
        search_scope = if ($null -ne $preferredRoot) { 'preferred_app_window' } else { 'active_window' }
        active_window = $activeWindow
        focused_element = Convert-UiElement($focused)
        candidates = @(Get-DiagnosticCandidates $candidateSearchRoot $selector)
        retry_hint = if ($selector.app_hint) {
            'The requested control was not found in the preferred app window. Try a more specific control name, or focus the correct window first.'
        } else {
            'Try inspecting accessibility for the current window, focusing the intended app first, or using a more specific control name.'
        }
    }
    throw ('UIA_BLOCKED:' + ($blocked | ConvertTo-Json -Depth 6 -Compress))
}
"#;
