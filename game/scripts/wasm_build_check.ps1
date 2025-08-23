<#
Minimal WASM build check script for Phase 7 metaballs feature.
Usage:
  pwsh ./game/scripts/wasm_build_check.ps1 [-FeatureSet <feature list>] [-NoInstall] [-Verbose]

Defaults:
  - Features: "metaballs"
  - Target: wasm32-unknown-unknown
Behavior:
  1. Ensures wasm target installed (unless -NoInstall).
  2. Performs a clean, quiet build of bevy_app with the selected features ONLY (no default features).
  3. Emits JSON summary to stdout.
Exit codes:
  0 success build
  1 failure (build or target install)
In CI you can parse the JSON line beginning with [wasm_check].
#>

param(
    [string]$FeatureSet = "metaballs",
    [switch]$NoInstall,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

function Write-Log($msg) {
    if ($Verbose) {
        Write-Host "[wasm_check][log] $msg"
    }
}

$target = "wasm32-unknown-unknown"
$startOverall = Get-Date

# Change directory to game workspace root (script lives in game/scripts)
try {
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $gameRoot = Split-Path -Parent $scriptDir
    Set-Location $gameRoot
} catch {
    Write-Host "[wasm_check] {`"ok`":false,`"error`":`"failed to change directory to game root: $($_.Exception.Message)`"}"
    exit 1
}

# Detect rustup
$r = Get-Command rustup -ErrorAction SilentlyContinue
if (-not $r) {
    Write-Host "[wasm_check] {`"ok`":false,`"error`":`"rustup not found in PATH`"}"
    exit 1
}

if (-not $NoInstall) {
    try {
        Write-Log "Ensuring target $target installed..."
        rustup target add $target | Out-Null
    } catch {
        $err = $_.Exception.Message
        $jsonErr = @{
            ok = $false
            error = "failed rustup target add $($target): $err"
        } | ConvertTo-Json -Compress
        Write-Host "[wasm_check] $jsonErr"
        exit 1
    }
} else {
    Write-Log "Skipping target install (NoInstall set)"
}

$buildStart = Get-Date
$featuresArg = $FeatureSet.Trim()
if ($featuresArg.Length -eq 0) {
    $featuresArg = "metaballs"
}

# Build command (no default features to ensure gating correctness)
# Use --manifest-path to avoid ambiguity with upstream bevy_app crate.
$cargoCmd = @(
    "cargo", "build",
    "--manifest-path", "crates/bevy_app/Cargo.toml",
    "--no-default-features",
    "--features", $featuresArg,
    "--target", $target,
    "--quiet"
)

Write-Log ("Running: " + ($cargoCmd -join " "))

$success = $true
$buildError = $null
try {
    & $cargoCmd[0] $cargoCmd[1..($cargoCmd.Length-1)]
} catch {
    $success = $false
    $buildError = $_.Exception.Message
}

$buildEnd = Get-Date

$durationInstallSec = [Math]::Round( ($buildStart - $startOverall).TotalSeconds, 3)
$durationBuildSec   = [Math]::Round( ($buildEnd - $buildStart).TotalSeconds, 3)
$totalSec           = [Math]::Round( ($buildEnd - $startOverall).TotalSeconds, 3)

if ($success) {
    $json = @{
        ok = $true
        target = $target
        features = $featuresArg
        duration_install_s = $durationInstallSec
        duration_build_s = $durationBuildSec
        total_s = $totalSec
        timestamp_utc = (Get-Date).ToUniversalTime().ToString("o")
    } | ConvertTo-Json -Compress
    Write-Host "[wasm_check] $json"
    exit 0
} else {
    $json = @{
        ok = $false
        target = $target
        features = $featuresArg
        error = $buildError
        duration_install_s = $durationInstallSec
        duration_build_s = $durationBuildSec
        total_s = $totalSec
        timestamp_utc = (Get-Date).ToUniversalTime().ToString("o")
    } | ConvertTo-Json -Compress
    Write-Host "[wasm_check] $json"
    exit 1
}
