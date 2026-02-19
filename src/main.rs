//! PC Unifier -- cross-platform input automation engine.
//!
//! Entry point, daemon lifecycle, and signal handling.

mod config;
mod engine;
mod event_bus;
mod lua_runtime;
#[allow(dead_code)]
mod platform;
mod rule_engine;

fn main() {
    println!("pcunifier v{}", env!("CARGO_PKG_VERSION"));
}
