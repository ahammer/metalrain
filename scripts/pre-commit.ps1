#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'
Write-Host '[pre-commit] cargo fmt --check'
cargo fmt --all -- --check
Write-Host '[pre-commit] cargo clippy'
cargo clippy --all-targets --all-features -- -D warnings
Write-Host '[pre-commit] cargo test'
cargo test --all --all-features --no-fail-fast
Write-Host '[pre-commit] OK'
