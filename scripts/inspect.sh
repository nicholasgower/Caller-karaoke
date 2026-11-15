#!/usr/bin/env bash
set -euo pipefail

export EGUI_INSPECTION="${EGUI_INSPECTION:-127.0.0.1:5719}"

echo "==> contra-app with inspection on ${EGUI_INSPECTION}"
cargo run -p contra-app --features inspection
