//! CPU profiler for every user-facing action.
//!
//! Run with:
//! `cargo run --release -p caller-app --features headless --bin caller-perf`
//!
//! Pass a sample count as the first argument to trade accuracy for time:
//! `... --bin caller-perf -- 10`.

fn main() {
    if let Err(error) = caller_app::app::perf::run() {
        eprintln!("performance profile failed: {error}");
        std::process::exit(1);
    }
}
