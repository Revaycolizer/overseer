pub const SCRIPT_PART1: &str = r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class OverseerWinApi {
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
}
"@
Add-Type -AssemblyName UIAutomationClient
$selector = '{selector_json}' | ConvertFrom-Json
$hwnd = [OverseerWinApi]::GetForegroundWindow()
$root = $null
try {
    if ($hwnd -ne [IntPtr]::Zero) {
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
    }
} catch {}
if ($null -eq $root) {
    $root = [System.Windows.Automation.AutomationElement]::RootElement
}
function Convert-UiElement($element) {
    if ($null -eq $element) { return $null }
    $rect = $null
    try { $rect = $element.Current.BoundingRectangle } catch {}
    [pscustomobject]@{
        name = try { $element.Current.Name } catch { $null }
        automation_id = try { $element.Current.AutomationId } catch { $null }
        class_name = try { $element.Current.ClassName } catch { $null }
        control_type = try { $element.Current.ControlType.ProgrammaticName } catch { $null }
        process_id = try { $element.Current.ProcessId } catch { $null }
        is_enabled = try { $element.Current.IsEnabled } catch { $null }
        has_keyboard_focus = try { $element.Current.HasKeyboardFocus } catch { $null }
        bounding_rectangle = if ($rect) {
            [pscustomobject]@{
                left = [int]$rect.Left
                top = [int]$rect.Top
                width = [int]$rect.Width
                height = [int]$rect.Height
            }
        } else { $null }
    }
}
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
"#;
