param(
    [string]$Version,
    [switch]$Push,
    [switch]$Commit
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

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

if (-not $Version) {
    $Version = Get-CargoVersion -CargoTomlPath (Join-Path $repoRoot "Cargo.toml")
}

$tag = "v$Version"
$existing = git tag --list $tag
if ($existing -eq $tag) {
    throw "Git tag $tag already exists."
}

if ($Commit) {
    Write-Host "[INFO] Staging release metadata..."
    git add (Join-Path $repoRoot "Cargo.toml")
    $notesDir = Join-Path $repoRoot "releases\notes"
    if (Test-Path $notesDir) {
        git add "$notesDir\"
    }

    $diff = git diff --cached --quiet
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[INFO] Committing release metadata for $tag..."
        git commit -m "chore: release $tag"
    } else {
        Write-Host "[INFO] No metadata changes to commit."
    }
}

Write-Host "[INFO] Creating git tag $tag..."
git tag -a $tag -m "Overseer $tag"
if ($LASTEXITCODE -ne 0) {
    throw "Failed to create git tag $tag"
}

if ($Push) {
    Write-Host "[INFO] Pushing changes and tag $tag to origin..."
    if ($Commit) {
        git push origin main
    }
    git push origin $tag
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to push git tag $tag"
    }
}

Write-Host "[INFO] Created git tag $tag"
