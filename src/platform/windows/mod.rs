//! Windows platform backend: WH_KEYBOARD_LL / WH_MOUSE_LL capture, SendInput injection.
//!
//! M5 milestone. Factory functions return boxed trait objects backed by
//! `WindowsCapture` (WH_KEYBOARD_LL) and `WindowsExecutor` (SendInput).

mod capture;
mod executor;
pub mod keycodes;

use capture::WindowsCapture;
use executor::WindowsExecutor;

use crate::platform::{ActionExecutor, InputCapture, PlatformError};

/// Returns a `WindowsCapture` backed by `WH_KEYBOARD_LL`.
pub fn create_input_capture() -> Result<Box<dyn InputCapture>, PlatformError> {
    Ok(Box::new(WindowsCapture::new()))
}

/// Returns a `WindowsExecutor` backed by `SendInput`.
pub fn create_action_executor() -> Result<Box<dyn ActionExecutor>, PlatformError> {
    Ok(Box::new(WindowsExecutor::new()))
}
