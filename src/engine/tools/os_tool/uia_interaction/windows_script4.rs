pub const SCRIPT_PART4: &str = r#"
$method = 'focus'
$verified = $false
$verification = 'not_checked'
$postFocused = $null
$postElement = $null
function Same-UiElement($left, $right) {
    if ($null -eq $left -or $null -eq $right) { return $false }
    try {
        if ($left.Current.NativeWindowHandle -ne 0 -and $left.Current.NativeWindowHandle -eq $right.Current.NativeWindowHandle) {
            return $true
        }
    } catch {}
    $leftName = (try { $left.Current.Name } catch { '' })
    $rightName = (try { $right.Current.Name } catch { '' })
    $leftAuto = (try { $left.Current.AutomationId } catch { '' })
    $rightAuto = (try { $right.Current.AutomationId } catch { '' })
    $leftType = (try { $left.Current.ControlType.ProgrammaticName } catch { '' })
    $rightType = (try { $right.Current.ControlType.ProgrammaticName } catch { '' })
    return $leftName -eq $rightName -and $leftAuto -eq $rightAuto -and $leftType -eq $rightType
}
if ($action -eq 'invoke_element') {
    $invoked = $false
    $toggleBefore = $null
    $toggleAfter = $null
    $expandBefore = $null
    $expandAfter = $null
    try {
        $pattern = $match.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        if ($pattern) {
            $pattern.Invoke()
            $method = 'invoke_pattern'
            $invoked = $true
        }
    } catch {}
    if (-not $invoked) {
        try {
            $pattern = $match.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)
            if ($pattern) {
                $pattern.Select()
                $method = 'selection_item_pattern'
                $invoked = $true
            }
        } catch {}
    }
    if (-not $invoked) {
        try {
            $pattern = $match.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
            if ($pattern) {
                $toggleBefore = [string]$pattern.Current.ToggleState
                $pattern.Toggle()
                $toggleAfter = [string]$pattern.Current.ToggleState
                $method = 'toggle_pattern'
                $invoked = $true
            }
        } catch {}
    }
    if (-not $invoked) {
        try {
            $pattern = $match.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern)
            if ($pattern) {
                $expandBefore = [string]$pattern.Current.ExpandCollapseState
                if ($expandBefore -eq 'Collapsed') {
                    $pattern.Expand()
                } else {
                    $pattern.Collapse()
                }
                $expandAfter = [string]$pattern.Current.ExpandCollapseState
                $method = 'expand_collapse_pattern'
                $invoked = $true
            }
        } catch {}
    }
    if (-not $invoked) {
        try {
            $pattern = $match.GetCurrentPattern([System.Windows.Automation.LegacyIAccessiblePattern]::Pattern)
            if ($pattern) {
                $pattern.DoDefaultAction()
                $method = 'legacy_default_action'
                $invoked = $true
            }
        } catch {}
    }
    if (-not $invoked) {
        $match.SetFocus()
        $method = 'focus_fallback'
    }
    try { $postFocused = [System.Windows.Automation.AutomationElement]::FocusedElement } catch {}
    $postElement = $match
    if ($method -eq 'selection_item_pattern') {
        try {
            $pattern = $match.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)
            $verified = [bool]$pattern.Current.IsSelected
            $verification = 'selection_item_selected'
        } catch {}
    } elseif ($method -eq 'toggle_pattern') {
        $verified = $toggleBefore -ne $toggleAfter -and $null -ne $toggleAfter
        $verification = if ($verified) { 'toggle_state_changed' } else { 'toggle_state_unknown' }
    } elseif ($method -eq 'expand_collapse_pattern') {
        $verified = $expandBefore -ne $expandAfter -and $null -ne $expandAfter
        $verification = if ($verified) { 'expand_collapse_state_changed' } else { 'expand_collapse_state_unknown' }
    } elseif ($method -eq 'invoke_pattern' -or $method -eq 'legacy_default_action' -or $method -eq 'focus_fallback') {
        $verified = Same-UiElement $match $postFocused
        $verification = if ($verified) { 'focused_after_invoke' } else { 'invoke_not_confirmed' }
    }
} elseif ($action -eq 'type_text') {
    $typed = $false
    $typedValue = $null
    try {
        $pattern = $match.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
        if ($pattern) {
            $pattern.SetValue($text)
            $typedValue = [string]$pattern.Current.Value
            $method = 'value_pattern'
            $typed = $true
        }
    } catch {}
    if (-not $typed) {
        try {
            $match.SetFocus()
            $method = 'focus_ready'
        } catch {
            throw 'Matched UI Automation element could not be prepared for text input.'
        }
    }
    try { $postFocused = [System.Windows.Automation.AutomationElement]::FocusedElement } catch {}
    $postElement = $match
    if ($method -eq 'value_pattern') {
        $verified = $typedValue -eq $text
        $verification = if ($verified) { 'value_pattern_roundtrip' } else { 'value_pattern_unconfirmed' }
    } else {
        $verified = Same-UiElement $match $postFocused
        $verification = if ($verified) { 'focused_for_keyboard_typing' } else { 'keyboard_typing_unconfirmed' }
    }
} else {
    $match.SetFocus()
    $method = 'focus'
    try { $postFocused = [System.Windows.Automation.AutomationElement]::FocusedElement } catch {}
    $postElement = $match
    $verified = Same-UiElement $match $postFocused
    $verification = if ($verified) { 'focused_element_confirmed' } else { 'focus_not_confirmed' }
}
[pscustomobject]@{
    action = $action
    selector = $selector
    search_scope = $searchScope
    method = $method
    element = Convert-UiElement($match)
    post_action_focused_element = Convert-UiElement($postFocused)
    post_action_element = Convert-UiElement($postElement)
    verification = $verification
    verified = $verified
    text_length = if ($action -eq 'type_text') { $text.Length } else { $null }
    ok = $true
} | ConvertTo-Json -Depth 6 -Compress
"#;
