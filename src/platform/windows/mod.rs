//! Windows platform backend: WH_KEYBOARD_LL / WH_MOUSE_LL capture, SendInput emulation.
//!
//! M5 milestone. Stubs are present so the binary compiles on Windows; they
//! return `PlatformError::Unavailable` until the implementation is complete.
//! Mouse capture (WH_MOUSE_LL) is deferred; the roadmap lists full backend work for M5.

use crate::platform::{ActionExecutor, InputCapture, PlatformError};

/// Placeholder until M5 implements the Windows capture backend.
pub fn create_input_capture() -> Result<Box<dyn InputCapture>, PlatformError> {
    Err(PlatformError::Unavailable(
        "Windows input capture is not yet implemented (M5).".into(),
    ))
}

/// Placeholder until M5 implements the Windows executor backend.
pub fn create_action_executor() -> Result<Box<dyn ActionExecutor>, PlatformError> {
    Err(PlatformError::Unavailable(
        "Windows action executor is not yet implemented (M5).".into(),
    ))
}
