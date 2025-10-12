param(
    [string]$Crate = "architecture_test"
)
Write-Host "Building WASM target for crate: $Crate" -ForegroundColor Cyan
$env:RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals"
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Error "rustup not found; install Rust toolchain first."; exit 1
}
# Ensure target installed (idempotent)
rustup target add wasm32-unknown-unknown | Out-Null
cargo build --target wasm32-unknown-unknown -p $Crate
if ($LASTEXITCODE -ne 0) { Write-Error "WASM build failed"; exit 1 }
Write-Host "WASM build complete: target/wasm32-unknown-unknown/debug" -ForegroundColor Green
