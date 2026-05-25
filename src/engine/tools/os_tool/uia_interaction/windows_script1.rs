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
$action = '{action_name}'
$text = '{text_value}'
$hwnd = [OverseerWinApi]::GetForegroundWindow()
$root = $null
try {
    if ($hwnd -ne [IntPtr]::Zero) {
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
    }
} catch {}
if ($null -eq $root) {
    throw 'No active window UI Automation root available.'
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
"#;
