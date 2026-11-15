$ErrorActionPreference = "Stop"

Write-Host "==> cargo fmt"
cargo fmt --all --check

Write-Host "==> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

Write-Host "==> cargo test"
cargo test --workspace

Write-Host "All checks passed."
