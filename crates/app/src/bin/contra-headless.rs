//! Headless documentation screenshot generator.
//!
//! Run with:
//! `cargo run -p contra-app --features headless --bin contra-headless -- website/static/img/screens`

fn main() {
    if let Err(error) = contra_app::app::headless::run() {
        eprintln!("headless screenshots failed: {error}");
        std::process::exit(1);
    }
}
