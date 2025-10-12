<#!
.SYNOPSIS
  WebGPU-only WASM dev helper for Ball Matcher.

.DESCRIPTION
  Builds and runs the project for target wasm32-unknown-unknown using wasm-server-runner.
  Optional: installs required target, wasm-server-runner, and (optionally) cargo-watch.
  If cargo-watch is present, enters watch mode (rebuilds on changes in src/, assets/, web/).
  Falls back to a single run when cargo-watch is not installed.
  Fails fast if web/index.html is missing.

.PARAMETER Release
  Use a release build (optimized) instead of debug.

.PARAMETER Install
  Ensure target + tools are installed before running.

.EXAMPLE
  pwsh scripts/wasm-dev.ps1 -Install

.EXAMPLE
  pwsh scripts/wasm-dev.ps1

.EXAMPLE
  pwsh scripts/wasm-dev.ps1 -Release
#>
[CmdletBinding()]
param(
  [switch]$Release,
  [switch]$Install,
  # Enable embedded shader feature for deterministic loads (no network fetch of WGSL)
  [switch]$Embed
)

$ErrorActionPreference = 'Stop'

function Write-Section {
  param([string]$Message)
  Write-Host "`n=== $Message ===" -ForegroundColor Cyan
}

function Write-Warn {
  param([string]$Message)
  Write-Host "[warn] $Message" -ForegroundColor Yellow
}

function Write-Err {
  param([string]$Message)
  Write-Host "[error] $Message" -ForegroundColor Red
}

function Resolve-RepoRoot {
  if ($PSScriptRoot) {
    return (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
  }
  # Fallback: current working directory
  return (Get-Location).Path
}

function Test-IndexHtml {
  param([string]$Root)
  $indexPath = Join-Path $Root 'web/index.html'
  if (-not (Test-Path $indexPath)) {
    Write-Err "Missing required web/index.html (expected at: $indexPath). Aborting."
    exit 1
  }
}

function Initialize-Target {
  Write-Section "Ensuring wasm32 target"
  $targets = (& rustup target list --installed)
  if ($targets -notcontains 'wasm32-unknown-unknown') {
    rustup target add wasm32-unknown-unknown
  } else {
    Write-Host "Target already installed."
  }
}

function Test-CommandAvailable {
  param([string]$Command)
  return [bool](Get-Command $Command -ErrorAction SilentlyContinue)
}

function Initialize-Tooling {
  Write-Section "Ensuring tooling"
  if (-not (Test-CommandAvailable wasm-server-runner)) {
    Write-Host "Installing wasm-server-runner..."
    cargo install wasm-server-runner
  } else {
    Write-Host "wasm-server-runner already installed."
  }

  if (-not (Test-CommandAvailable cargo-watch)) {
    Write-Warn "cargo-watch not installed (optional). Install with: cargo install cargo-watch"
  } else {
    Write-Host "cargo-watch detected."
  }
}

function Invoke-WasmRun {
  param(
    [switch]$ReleaseBuild,
    [switch]$UseWatch,
    [string]$FeaturesFlag
  )
  # Ensure cargo uses wasm-server-runner to serve and execute the produced .wasm instead of
  # trying to run the raw wasm file (which fails on Windows with os error 193).
  if (-not $env:CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER) {
    $env:CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER = 'wasm-server-runner'
  }
  $profileFlag = $ReleaseBuild.IsPresent ? "--release" : ""
  $target = "wasm32-unknown-unknown"
  $package = "metaballs_test"

  if ($UseWatch -and (Test-CommandAvailable cargo-watch)) {
    Write-Section "Starting watch mode (src, assets, web)"
    cargo watch `
      -w demos/metaballs_test/src `
      -w crates/metaball_renderer/src `
      -w assets `
      -w web `
      -x "run --package $package --target $target $profileFlag $FeaturesFlag" `
      --why
  } else {
    if ($UseWatch -and -not (Test-CommandAvailable cargo-watch)) {
      Write-Warn "Watch requested but cargo-watch missing; performing single run."
    }
    Write-Section "Running (single invocation)"
    cargo run --package $package --target $target $profileFlag $FeaturesFlag
  }
}

function Initialize-WebGpuNotice {
  Write-Host "WebGPU-only: Requires a modern browser with navigator.gpu (Chrome ≥113, Edge, Firefox Nightly w/ flag, or Safari TP)." -ForegroundColor Green
  Write-Host "No WebGL fallback is provided; unsupported browsers will fail early." -ForegroundColor Green
}

$root = Resolve-RepoRoot
Set-Location $root

Test-IndexHtml -Root $root
Initialize-WebGpuNotice

if ($Install) {
  Initialize-Target
  Initialize-Tooling
}

if ($Install -eq $false) {
  $targets = (& rustup target list --installed)
  if ($targets -notcontains 'wasm32-unknown-unknown') {
    Write-Warn "Target wasm32-unknown-unknown missing; adding automatically."
    Initialize-Target
  }
}

$watch = $true
if ($Release) {
  Write-Warn "Release watch disabled for performance; doing single optimized run."
  $watch = $false
}

$featuresFlag = ""
if ($Embed) { $featuresFlag = "--features metaball_renderer/embed_shaders" }


Invoke-WasmRun -ReleaseBuild:$Release -UseWatch:$watch -FeaturesFlag:$featuresFlag
