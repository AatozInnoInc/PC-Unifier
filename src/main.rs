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

use crate::platform::{create_action_executor, create_input_capture, Action, PlatformError};

fn main() -> Result<(), PlatformError> {
    // Initialize logger. Default level: info. Override with RUST_LOG=debug for latency output.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("pcunifier v{}", env!("CARGO_PKG_VERSION"));

    let mut capture = create_input_capture()?;
    let executor = create_action_executor()?;
    capture.start(Box::new(move |event| {
        let _ = executor.execute(&Action::InjectKey { key: event.key, state: event.state });
    }))?;

    // Block until process is terminated (e.g. Ctrl+C or SIGTERM).
    loop {
        std::thread::sleep(std::time::Duration::from_secs(86400));
    }
}