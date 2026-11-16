$ErrorActionPreference = "Stop"

# Build the profiler in release mode: a debug profile is several times slower
# and would flag functions that are fine in a shipped build. Pass a sample
# count as the first argument, e.g. `scripts\profile.ps1 10`.
cargo run --release -p contra-app --features headless --bin contra-perf -- @args
