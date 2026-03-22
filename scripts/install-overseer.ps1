param(
    [string]$SourceDir,
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Overseer",
    [switch]$NoDesktopShortcut
)

$ErrorActionPreference = "Stop"

function New-Shortcut {
    param(
        [string]$ShortcutPath,
        [string]$TargetPath,
        [string]$WorkingDirectory,
        [string]$Arguments = ""
    )

    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($ShortcutPath)
    $shortcut.TargetPath = $TargetPath
    $shortcut.WorkingDirectory = $WorkingDirectory
    if ($Arguments) {
        $shortcut.Arguments = $Arguments
    }
    $shortcut.IconLocation = $TargetPath
    $shortcut.Save()
}

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $SourceDir) {
    $portableSibling = Join-Path $scriptRoot "portable"
    if (Test-Path (Join-Path $portableSibling "Overseer.exe")) {
        $SourceDir = $portableSibling
    } else {
        $SourceDir = $scriptRoot
    }
}

$sourceExe = Join-Path $SourceDir "Overseer.exe"
if (-not (Test-Path $sourceExe)) {
    throw "Could not find Overseer.exe under $SourceDir"
}

Write-Host "[INFO] Installing Overseer to $InstallDir"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item -Path (Join-Path $SourceDir "*") -Destination $InstallDir -Recurse -Force

$siblingUninstall = Join-Path $scriptRoot "uninstall-overseer.ps1"
if (Test-Path $siblingUninstall) {
    Copy-Item -Path $siblingUninstall -Destination (Join-Path $InstallDir "uninstall-overseer.ps1") -Force
}

$installedExe = Join-Path $InstallDir "Overseer.exe"
$installedUninstall = Join-Path $InstallDir "uninstall-overseer.ps1"
$desktopPath = [Environment]::GetFolderPath("Desktop")
$startMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Overseer"
New-Item -ItemType Directory -Force -Path $startMenuDir | Out-Null

New-Shortcut -ShortcutPath (Join-Path $startMenuDir "Overseer.lnk") -TargetPath $installedExe -WorkingDirectory $InstallDir
if (Test-Path $installedUninstall) {
    New-Shortcut `
        -ShortcutPath (Join-Path $startMenuDir "Uninstall Overseer.lnk") `
        -TargetPath "powershell.exe" `
        -WorkingDirectory $InstallDir `
        -Arguments "-ExecutionPolicy Bypass -File `"$installedUninstall`""
}

if (-not $NoDesktopShortcut) {
    New-Shortcut -ShortcutPath (Join-Path $desktopPath "Overseer.lnk") -TargetPath $installedExe -WorkingDirectory $InstallDir
}

Write-Host "[INFO] Overseer installed successfully."
