use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::powershell::run_powershell_hidden;

pub async fn windows_desktop_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let script = r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class OverseerWinApi {
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
}
"@
Add-Type -AssemblyName UIAutomationClient
$hwnd = [OverseerWinApi]::GetForegroundWindow()
$activeTitle = ""
$activePid = 0
if ($hwnd -ne [IntPtr]::Zero) {
    $sb = New-Object System.Text.StringBuilder 1024
    [void][OverseerWinApi]::GetWindowText($hwnd, $sb, $sb.Capacity)
    $activeTitle = $sb.ToString()
    [void][OverseerWinApi]::GetWindowThreadProcessId($hwnd, [ref]$activePid)
}
$activeElement = $null
try {
    if ($hwnd -ne [IntPtr]::Zero) {
        $activeElement = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
    }
} catch {}
$focusedElement = $null
try {
    $focusedElement = [System.Windows.Automation.AutomationElement]::FocusedElement
} catch {}
function Convert-UiElement($element) {
    if ($null -eq $element) { return $null }
    $rect = $null
    try { $rect = $element.Current.BoundingRectangle } catch {}
    $e_name = $null; try { $e_name = $element.Current.Name } catch {}
    $e_aid = $null; try { $e_aid = $element.Current.AutomationId } catch {}
    $e_class = $null; try { $e_class = $element.Current.ClassName } catch {}
    $e_ctrl = $null; try { $e_ctrl = $element.Current.ControlType.ProgrammaticName } catch {}
    $e_pid = $null; try { $e_pid = $element.Current.ProcessId } catch {}
    $e_enabled = $null; try { $e_enabled = $element.Current.IsEnabled } catch {}
    $e_focus = $null; try { $e_focus = $element.Current.HasKeyboardFocus } catch {}
    [pscustomobject]@{
        name = $e_name
        automation_id = $e_aid
        class_name = $e_class
        control_type = $e_ctrl
        process_id = $e_pid
        is_enabled = $e_enabled
        has_keyboard_focus = $e_focus
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
$uiaElements = @()
if ($activeElement -ne $null) {
    try {
        $descendants = $activeElement.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        $max = [Math]::Min($descendants.Count, 40)
        for ($i = 0; $i -lt $max; $i++) {
            $uiaElements += Convert-UiElement($descendants.Item($i))
        }
    } catch {}
}
$windows = Get-Process |
    Where-Object { $_.MainWindowTitle -and $_.MainWindowTitle.Trim().Length -gt 0 } |
    Select-Object -First 12 ProcessName, Id, MainWindowTitle
[pscustomobject]@{
    active_window_title = $activeTitle
    active_pid = $activePid
    visible_windows = $windows
    accessibility = [pscustomobject]@{
        active_window = Convert-UiElement($activeElement)
        focused_element = Convert-UiElement($focusedElement)
        elements = $uiaElements
        element_count = @($uiaElements).Count
    }
} | ConvertTo-Json -Depth 6 -Compress
"#;

    run_powershell_hidden(script, tool_name).await
}
