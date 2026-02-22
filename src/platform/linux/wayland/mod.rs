//! Wayland platform backend: RemoteDesktop portal (injection).
//!
//! The InputCapture portal path (`capture` module) is retained for reference
//! but is not active; keyboard capture now uses the evdev backend instead.

#[allow(dead_code)]
mod capture;
mod executor;

pub use executor::LinuxWaylandExecutor;
