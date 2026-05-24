param(
    [string]$Version,
    [string]$Tag,
    [string]$Channel = "stable",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

function Get-CargoVersion {
    param([string]$CargoTomlPath)

    $match = Select-String -Path $CargoTomlPath -Pattern '^version = "([^"]+)"' | Select-Object -First 1
    if (-not $match) {
        throw "Could not determine package version from $CargoTomlPath"
    }

    return $match.Matches[0].Groups[1].Value
}

function Copy-RequiredItem {
    param(
        [string]$Source,
        [string]$Destination
    )

    if (-not (Test-Path $Source)) {
        throw "Required release asset not found: $Source"
    }

    Copy-Item -Path $Source -Destination $Destination -Recurse -Force
}

function Resolve-FirstExistingPath {
    param(
        [string[]]$Candidates
    )

    foreach ($candidate in $Candidates) {
        if (Test-Path $candidate) {
            return $candidate
        }
    }

    return $null
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoToml = Join-Path $repoRoot "Cargo.toml"

if (-not $Version) {
    $Version = Get-CargoVersion -CargoTomlPath $cargoToml
}

if (-not $Tag) {
    $Tag = "v$Version"
}

$distRoot = Join-Path $repoRoot "dist"
$releaseRoot = Join-Path $distRoot $Tag
$portableRoot = Join-Path $releaseRoot "portable"
$zipName = "Overseer-$Tag-windows-x64.zip"
$zipPath = Join-Path $releaseRoot $zipName
$setupExeName = "Overseer-$Tag-windows-x64-setup.exe"
$setupExePath = Join-Path $releaseRoot $setupExeName
$manifestPath = Join-Path $releaseRoot "release-windows-x64.json"
$checksumsPath = Join-Path $releaseRoot "SHA256SUMS-windows-x64.txt"
$dxAppRoot = Join-Path $repoRoot "target\dx\overseer\release\windows\app"
$dxExeSource = Join-Path $dxAppRoot "overseer.exe"
$dxAssetsSource = Join-Path $dxAppRoot "assets"
$exeSource = $dxExeSource
$toolScriptsSource = Resolve-FirstExistingPath -Candidates @(
    (Join-Path $repoRoot "tool_scripts"),
    (Join-Path $repoRoot "src\engine\scripts"),
    (Join-Path $repoRoot "agents-scripts")
)
$agentSeedsSource = Resolve-FirstExistingPath -Candidates @(
    (Join-Path $repoRoot "src\engine\agents"),
    (Join-Path $repoRoot "config\agents")
)
$releaseNotesSource = Join-Path $repoRoot "releases\notes\$Tag.md"

if (-not $SkipBuild) {
    Write-Host "[INFO] Building Overseer $Tag..."
    dx build --platform desktop --release
    if ($LASTEXITCODE -ne 0) {
        throw "dx build failed with exit code $LASTEXITCODE"
    }
}

if (-not (Test-Path $exeSource)) {
    throw "Compiled executable not found at $exeSource"
}
if (-not (Test-Path $dxAssetsSource)) {
    throw "Bundled desktop assets not found at $dxAssetsSource"
}

if (Test-Path $releaseRoot) {
    Remove-Item -Path $releaseRoot -Recurse -Force
}

New-Item -ItemType Directory -Path $portableRoot -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $portableRoot "src\engine") -Force | Out-Null

Write-Host "[INFO] Staging portable package..."
Copy-RequiredItem -Source $exeSource -Destination (Join-Path $portableRoot "Overseer.exe")
Copy-RequiredItem -Source $dxAssetsSource -Destination (Join-Path $portableRoot "assets")
Copy-RequiredItem -Source (Join-Path $repoRoot "config") -Destination (Join-Path $portableRoot "config")
if ($toolScriptsSource) {
    Copy-RequiredItem -Source $toolScriptsSource -Destination (Join-Path $portableRoot "tool_scripts")
} else {
    Write-Host "[WARN] No tool scripts directory found. Continuing without packaged Python tool sources."
}
if ($agentSeedsSource) {
    Copy-RequiredItem -Source $agentSeedsSource -Destination (Join-Path $portableRoot "src\engine\agents")
} else {
    Write-Host "[WARN] No code-based agent seed directory found. Continuing with packaged config agents only."
}
Copy-RequiredItem -Source (Join-Path $repoRoot "CHANGELOG.md") -Destination (Join-Path $portableRoot "CHANGELOG.md")
Copy-RequiredItem -Source (Join-Path $repoRoot "README.md") -Destination (Join-Path $portableRoot "README.md")

if (Test-Path $releaseNotesSource) {
    Copy-RequiredItem -Source $releaseNotesSource -Destination (Join-Path $portableRoot "$Tag.md")
}

Copy-RequiredItem -Source (Join-Path $repoRoot "scripts\install-overseer.ps1") -Destination (Join-Path $releaseRoot "install-overseer.ps1")
Copy-RequiredItem -Source (Join-Path $repoRoot "scripts\uninstall-overseer.ps1") -Destination (Join-Path $releaseRoot "uninstall-overseer.ps1")

if (Test-Path $zipPath) {
    Remove-Item $zipPath -Force
}

Compress-Archive -Path (Join-Path $portableRoot "*") -DestinationPath $zipPath -CompressionLevel Optimal

$makensis = Get-Command makensis -ErrorAction SilentlyContinue
if ($makensis) {
    $makensisPath = $makensis.Source
} else {
    $candidates = @(
        "C:\Program Files (x86)\NSIS\makensis.exe",
        "C:\Program Files\NSIS\makensis.exe"
    )
    $makensisPath = Resolve-FirstExistingPath -Candidates $candidates
}

if (-not $makensisPath) {
    throw "NSIS Compiler (makensis.exe) not found. Please install NSIS (e.g., run `winget install NSIS.NSIS`) or add its folder to your system PATH."
}

Write-Host "[INFO] Generating installer visual assets..."
try {
    & python (Join-Path $repoRoot "scripts\generate_installer_assets.py")
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[WARN] Dynamic asset generation failed with exit code $LASTEXITCODE. Using committed pre-generated assets."
    }
} catch {
    Write-Host "[WARN] Dynamic asset generation failed: $_. Using committed pre-generated assets."
}

Write-Host "[INFO] Compiling production-grade setup installer using NSIS..."
& $makensisPath /DVERSION="$Version" /DTAG="$Tag" /DREPO_ROOT="$repoRoot" /DOUT_DIR="$releaseRoot" /DPORTABLE_DIR="$portableRoot" (Join-Path $repoRoot "scripts\installer.nsi")
if ($LASTEXITCODE -ne 0) {
    throw "NSIS compilation failed with exit code $LASTEXITCODE"
}

$assets = @(
    (Get-Item $setupExePath),
    (Get-Item $zipPath),
    (Get-Item (Join-Path $releaseRoot "install-overseer.ps1")),
    (Get-Item (Join-Path $releaseRoot "uninstall-overseer.ps1"))
)

$hashLines = foreach ($asset in $assets) {
    $hash = (Get-FileHash -Algorithm SHA256 -Path $asset.FullName).Hash.ToLowerInvariant()
    "{0}  {1}" -f $hash, $asset.Name
}

Set-Content -Path $checksumsPath -Value $hashLines

$manifest = [ordered]@{
    product = "Overseer"
    version = $Version
    tag = $Tag
    channel = $Channel
    built_at = (Get-Date).ToUniversalTime().ToString("o")
    platform = "windows-x64"
    assets = @(
        @{
            name = $setupExeName
            kind = "setup-exe"
            sha256 = (Get-FileHash -Algorithm SHA256 -Path $setupExePath).Hash.ToLowerInvariant()
            size = (Get-Item $setupExePath).Length
        },
        @{
            name = $zipName
            kind = "portable-zip"
            sha256 = (Get-FileHash -Algorithm SHA256 -Path $zipPath).Hash.ToLowerInvariant()
            size = (Get-Item $zipPath).Length
        },
        @{
            name = "install-overseer.ps1"
            kind = "installer-script"
            sha256 = (Get-FileHash -Algorithm SHA256 -Path (Join-Path $releaseRoot "install-overseer.ps1")).Hash.ToLowerInvariant()
            size = (Get-Item (Join-Path $releaseRoot "install-overseer.ps1")).Length
        },
        @{
            name = "uninstall-overseer.ps1"
            kind = "uninstaller-script"
            sha256 = (Get-FileHash -Algorithm SHA256 -Path (Join-Path $releaseRoot "uninstall-overseer.ps1")).Hash.ToLowerInvariant()
            size = (Get-Item (Join-Path $releaseRoot "uninstall-overseer.ps1")).Length
        }
    )
}

$manifest | ConvertTo-Json -Depth 6 | Set-Content -Path $manifestPath

Write-Host "[INFO] Release ready at $releaseRoot"
