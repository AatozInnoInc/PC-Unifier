//! Platform abstraction layer.
//!
//! Defines the InputCapture and ActionExecutor traits (M2).
//! Platform-specific implementations live in child modules.

mod linux;
mod macos;
mod windows;
