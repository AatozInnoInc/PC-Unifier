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

use crate::platform::{create_action_executor, create_input_capture, PlatformError};

fn main() -> Result<(), PlatformError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("pcunifier v{}", env!("CARGO_PKG_VERSION"));

    // Load config; a missing file is normal on first run (full UX in M14).
    let config_path = config::default_config_path();
    let cfg = match config::load(&config_path) {
        Ok(c) => {
            log::info!("config: loaded from {}", config_path.display());
            c
        }
        Err(config::ConfigError::Io { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            log::info!(
                "config: no config file at {}, starting with empty ruleset",
                config_path.display()
            );
            config::Config::default()
        }
        Err(e) => return Err(PlatformError::Config(e.to_string())),
    };

    let rule_engine = rule_engine::RuleEngine::new(&cfg);

    let (publisher, subscriber) = event_bus::new(event_bus::DEFAULT_CAPACITY);

    let mut capture = create_input_capture()?;
    let executor = create_action_executor()?;

    capture.start(Box::new(move |event| {
        publisher.send(event);
    }))?;

    for event in subscriber {
        let action = rule_engine.process(&event);
        if let Err(e) = executor.execute(&action) {
            log::warn!("executor: inject failed: {e}");
        }
    }

    Ok(())
}
