//! Headless documentation screenshot generator.
//!
//! Run with:
//! `cargo run -p contra-app --features headless --bin contra-headless -- website/static/img/screens`
//!
//! Add `mobile` as a second argument to render the phone inspection set into
//! `<out>/mobile` without touching the documentation manifest.

fn main() {
    if let Err(error) = contra_app::app::headless::run() {
        eprintln!("headless screenshots failed: {error}");
        std::process::exit(1);
    }
}
