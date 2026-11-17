$ErrorActionPreference = "Stop"

if (-not $env:EGUI_INSPECTION) {
    $env:EGUI_INSPECTION = "127.0.0.1:5719"
}

Write-Host "==> caller-app with inspection on $env:EGUI_INSPECTION"
cargo run -p caller-app --features inspection
