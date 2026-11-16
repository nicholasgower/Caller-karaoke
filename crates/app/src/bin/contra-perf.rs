//! CPU profiler for every user-facing action.
//!
//! Run with:
//! `cargo run --release -p contra-app --features headless --bin contra-perf`
//!
//! Pass a sample count as the first argument to trade accuracy for time:
//! `... --bin contra-perf -- 10`.

fn main() {
    if let Err(error) = contra_app::app::perf::run() {
        eprintln!("performance profile failed: {error}");
        std::process::exit(1);
    }
}
