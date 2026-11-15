#!/usr/bin/env bash
set -euo pipefail

echo "==> cargo fmt"
cargo fmt --all --check

echo "==> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo check (inspection feature)"
cargo check -p contra-app --features inspection

echo "==> cargo test"
cargo test --workspace

echo "All checks passed."
