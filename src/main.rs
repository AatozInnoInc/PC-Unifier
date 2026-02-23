//! PC Unifier -- cross-platform input automation engine.
//!
//! Entry point, daemon lifecycle, and signal handling.

#[allow(dead_code)]
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

    let (publisher, subscriber) = event_bus::new(event_bus::DEFAULT_CAPACITY);

    let mut capture = create_input_capture()?;
    let executor = create_action_executor()?;

    // Capture callback publishes raw events onto the bus.
    capture.start(Box::new(move |event| {
        publisher.send(event);
    }))?;

    // Consumer loop: drain the bus and pass each event to the executor.
    // Exits when all publishers are dropped (i.e. capture stops cleanly).
    for event in subscriber {
        let _ = executor.execute(&Action::InjectKey {
            key: event.key,
            state: event.state,
        });
    }

    Ok(())
}
