param([string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Overseer\bin'))
$ErrorActionPreference = 'Stop'
if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() -ne 'X64') {
    throw 'This archive requires Windows x64.'
}
if ((Get-Content (Join-Path $PSScriptRoot 'TARGET') -Raw).Trim() -ne 'x86_64-pc-windows-msvc') {
    throw 'Download the Windows x64 archive.'
}
Get-Content (Join-Path $PSScriptRoot 'CHECKSUMS.txt') | ForEach-Object {
    $parts = $_ -split '  ', 2
    if ($parts.Count -ne 2 -or $parts[1] -match '[/\\]' -or $parts[1] -eq '..') { throw 'Invalid checksum entry.' }
    $actual = (Get-FileHash (Join-Path $PSScriptRoot $parts[1]) -Algorithm SHA256).Hash
    if ($actual -ne $parts[0]) { throw "Checksum mismatch: $($parts[1])" }
}
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item (Join-Path $PSScriptRoot 'overseer-tui.exe') (Join-Path $InstallDir 'overseer-tui.exe') -Force
Write-Host "Installed: $InstallDir\overseer-tui.exe"
Write-Host 'Add this directory to your user PATH, then run overseer-tui.'
Write-Host 'Uninstall by removing the executable; your data is retained.'
