pub const SCRIPT_PART2: &str = r#"
function Matches-Selector($element, $selector) {
    if ($null -eq $element) { return $false }
    $name = (try { $element.Current.Name } catch { '' })
    $automationId = (try { $element.Current.AutomationId } catch { '' })
    $className = (try { $element.Current.ClassName } catch { '' })
    $controlType = (try { $element.Current.ControlType.ProgrammaticName } catch { '' })
    if ($selector.name -and $name -notlike ('*' + $selector.name + '*')) { return $false }
    if ($selector.automation_id -and $automationId -notlike ('*' + $selector.automation_id + '*')) { return $false }
    if ($selector.class_name -and $className -notlike ('*' + $selector.class_name + '*')) { return $false }
    if ($selector.control_type -and $controlType -notlike ('*' + $selector.control_type + '*')) { return $false }
    if ($selector.selector) {
        $needle = [string]$selector.selector
        if ($name -notlike ('*' + $needle + '*') -and
            $automationId -notlike ('*' + $needle + '*') -and
            $className -notlike ('*' + $needle + '*') -and
            $controlType -notlike ('*' + $needle + '*')) {
            return $false
        }
    }
    return $true
}
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
function Get-DiagnosticCandidates($searchRoot, $selector) {
    $candidates = @()
    try {
        $all = $searchRoot.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        $max = [Math]::Min($all.Count, 160)
        for ($i = 0; $i -lt $max; $i++) {
            $candidate = $all.Item($i)
            $converted = Convert-UiElement($candidate)
            if ($null -eq $converted) { continue }
            $score = 0
            if ($selector.name -and $converted.name -like ('*' + $selector.name + '*')) { $score += 6 }
            if ($selector.selector -and $converted.name -like ('*' + $selector.selector + '*')) { $score += 4 }
            if ($selector.automation_id -and $converted.automation_id -like ('*' + $selector.automation_id + '*')) { $score += 5 }
            if ($selector.class_name -and $converted.class_name -like ('*' + $selector.class_name + '*')) { $score += 3 }
            if ($selector.control_type -and $converted.control_type -like ('*' + $selector.control_type + '*')) { $score += 4 }
            if ($score -gt 0 -or ($candidates.Count -lt 8 -and $converted.name)) {
                $converted | Add-Member -NotePropertyName score -NotePropertyValue $score
                $candidates += $converted
            }
        }
    } catch {}
    return $candidates | Sort-Object -Property score -Descending | Select-Object -First 8
}
"#;
