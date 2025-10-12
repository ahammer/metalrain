<#!
.SYNOPSIS
  WebGPU-only WASM dev helper for the bouncing_balls demo.

.DESCRIPTION
  Builds and runs ONLY the demo crate located at demos/bouncing_balls for target wasm32-unknown-unknown using wasm-server-runner.
  Optional: installs required target, wasm-server-runner, and (optionally) cargo-watch.
  If cargo-watch is present, enters watch mode (rebuilds on changes in the demo's src/, shared assets/, and root web/).
  Falls back to a single run when cargo-watch is not installed.
  Fails fast if required files are missing (web/index.html, demo Cargo.toml).

.PARAMETER Release
  Use a release build (optimized) instead of debug.

.PARAMETER Install
  Ensure target + tools are installed before running.

.EXAMPLE
  pwsh scripts/ball_bouncer_wasm.ps1 -Install

.EXAMPLE
  pwsh scripts/ball_bouncer_wasm.ps1

.EXAMPLE
  pwsh scripts/ball_bouncer_wasm.ps1 -Release
#>
[CmdletBinding()]
param(
  [switch]$Release,
  [switch]$Install
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

function Test-DemoCrateExists {
  param([string]$Root, [string]$RelPath)
  $manifest = Join-Path $Root (Join-Path $RelPath 'Cargo.toml')
  if (-not (Test-Path $manifest)) {
    Write-Err "Demo crate manifest not found at $manifest. Aborting."
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
    [string]$DemoRelPath,
    [string]$PackageName
  )
  $profileFlag = $ReleaseBuild.IsPresent ? "--release" : ""
  $target = "wasm32-unknown-unknown"

  # Build/run from workspace root, specifying package so shared assets & feature flags still resolve.
  if ($UseWatch -and (Test-CommandAvailable cargo-watch)) {
    Write-Section "Starting watch mode (demo src, assets, web)"
    cargo watch `
      -w "$DemoRelPath/src" `
      -w assets `
      -w web `
      -x "run -p $PackageName --target $target $profileFlag" `
      --why
  } else {
    if ($UseWatch -and -not (Test-CommandAvailable cargo-watch)) {
      Write-Warn "Watch requested but cargo-watch missing; performing single run."
    }
    Write-Section "Running demo (single invocation)"
    cargo run -p $PackageName --target $target $profileFlag
  }
}

function Initialize-WebGpuNotice {
  Write-Host "WebGPU-only: Requires a modern browser with navigator.gpu (Chrome ≥113, Edge, Firefox Nightly w/ flag, or Safari TP)." -ForegroundColor Green
  Write-Host "No WebGL fallback is provided; unsupported browsers will fail early." -ForegroundColor Green
}

$root = Resolve-RepoRoot
Set-Location $root

# Constants for this script (demo specifics)
$demoRelPath = 'demos/bouncing_balls'
$demoPackage = 'bouncing_balls'

Test-IndexHtml -Root $root
Test-DemoCrateExists -Root $root -RelPath $demoRelPath
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

Invoke-WasmRun -ReleaseBuild:$Release -UseWatch:$watch -DemoRelPath:$demoRelPath -PackageName:$demoPackage
