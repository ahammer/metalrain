<#!
.SYNOPSIS
Generate (and optionally inspect) the SDF atlas used by the project.

.DESCRIPTION
Wraps the Rust helper binaries `sdf_atlas_build` and `sdf_atlas_inspect` providing sane defaults
matching the committed atlas. Supports overriding key parameters for iteration.

.EXAMPLES
  # Rebuild with defaults (64 tile, padding 8, distance span 0.5) then inspect
  ./gen-atlas.ps1

  # Rebuild without inspection
  ./gen-atlas.ps1 -NoInspect

  # Change distance span and supersampling
  ./gen-atlas.ps1 -DistanceSpanFactor 0.6 -Supersamples 4

  # Quick help
  ./gen-atlas.ps1 -?

.NOTES
Rebuilds in release mode (for speed of glyph path processing) and overwrites:
  assets/shapes/sdf_atlas.png
  assets/shapes/sdf_atlas.json

The script will fail fast on any non‑zero cargo exit code.
#>
[CmdletBinding()] param(
  [int]$TileSize = 64,
  [int]$PaddingPx = 8,
  [double]$DistanceSpanFactor = 0.5,
  [ValidateSet(1,2,4)][int]$Supersamples = 1,
  [string]$ChannelMode = 'sdf_r8',
  [string]$Font = 'assets/fonts/DroidSansMono.ttf',
  [switch]$NoInspect,
  [switch]$SkipBuild,
  [switch]$VerboseLogging
)

$ErrorActionPreference = 'Stop'
$script:Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $Root

function Write-Info($msg) { if ($VerboseLogging) { Write-Host "[atlas] $msg" -ForegroundColor Cyan } }

$png = 'assets/shapes/sdf_atlas.png'
$json = 'assets/shapes/sdf_atlas.json'

if (-not $SkipBuild) {
  Write-Host "==> Building SDF atlas ($TileSize px tiles, padding $PaddingPx, span factor $DistanceSpanFactor, supersamples $Supersamples)" -ForegroundColor Green
  $args = @(
    'run','--release','--bin','sdf_atlas_build','--',
    '--out-png', $png,
    '--out-json', $json,
    '--tile-size', $TileSize,
    '--padding-px', $PaddingPx,
    '--distance-span-factor', $DistanceSpanFactor,
    '--supersamples', $Supersamples,
    '--channel-mode', $ChannelMode,
    '--font', $Font
  )
  Write-Info ("cargo " + ($args -join ' '))
  cargo @args
}
else {
  Write-Host "==> Skipping build step (using existing atlas)" -ForegroundColor Yellow
}

if (-not $NoInspect) {
  Write-Host "==> Inspecting atlas" -ForegroundColor Green
  $inspectArgs = @('run','--release','--bin','sdf_atlas_inspect','--', '--atlas-png', $png, '--atlas-json', $json)
  Write-Info ("cargo " + ($inspectArgs -join ' '))
  cargo @inspectArgs
} else {
  Write-Info 'Inspection skipped'
}

Pop-Location
