param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Overseer"
)

$ErrorActionPreference = "Stop"

$startMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Overseer"
$desktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "Overseer.lnk"

if (Test-Path $desktopShortcut) {
    Remove-Item $desktopShortcut -Force
}

if (Test-Path $startMenuDir) {
    Remove-Item $startMenuDir -Recurse -Force
}

if (Test-Path $InstallDir) {
    Remove-Item $InstallDir -Recurse -Force
}

Write-Host "[INFO] Overseer uninstalled."
