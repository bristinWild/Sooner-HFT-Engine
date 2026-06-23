//! Logging initialiser — call `init()` first in every binary.
//!
//! Control with env vars:
//!   RUST_LOG=debug          — log level per crate
//!   LOG_FORMAT=json         — structured JSON output (for production)

use tracing_subscriber::{fmt, EnvFilter};

pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let json = std::env::var("LOG_FORMAT")
        .map(|v| v.to_lowercase() == "json")
        .unwrap_or(false);

    if json {
        fmt()
            .json()
            .with_env_filter(filter)
            .with_current_span(true)
            .init();
    } else {
        fmt()
            .pretty()
            .with_env_filter(filter)
            .with_file(true)
            .with_line_number(true)
            .init();
    }
}
