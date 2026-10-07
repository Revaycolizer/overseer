use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::powershell::run_powershell_hidden;
use crate::engine::tools::vision::runtime_artifacts_dir;

pub async fn capture_windows_screenshot_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
    let output_path = runtime_artifacts_dir()?.join(format!("desktop-{}.png", timestamp));
    let output_path_str = output_path.to_string_lossy();

    let script = format!(
        r#"
[Reflection.Assembly]::LoadWithPartialName("System.Drawing") | Out-Null
[Reflection.Assembly]::LoadWithPartialName("System.Windows.Forms") | Out-Null
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bmp)
$graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$bmp.Save("{path}", [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bmp.Dispose()
$base64 = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes("{path}"))
@{{
    "status" = "success"
    "path" = "{path}"
    "mime" = "image/png"
    "base64" = $base64
}} | ConvertTo-Json -Compress
"#,
        path = output_path_str
    );

    run_powershell_hidden(&script, tool_name).await
}
