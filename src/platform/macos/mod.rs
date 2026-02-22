//! macOS platform backend.
//!
//! Capture: CGEventTap (HID level) via `MacOSCapture`.
//! Injection: CGEventPost (synchronous) via `MacOSExecutor`.
//!
//! Both backends require Accessibility permission. `MacOSCapture::start()`
//! calls `AXIsProcessTrusted()` and returns `PlatformError::PermissionDenied`
//! if permission has not been granted. Guide the user to:
//!   System Settings > Privacy & Security > Accessibility

mod capture;
mod executor;
mod keycodes;

use capture::MacOSCapture;
use executor::MacOSExecutor;

use crate::platform::{ActionExecutor, InputCapture, PlatformError};

// ---------------------------------------------------------------------------
// Factory: input capture
// ---------------------------------------------------------------------------

/// Returns the CGEventTap-based keyboard capture backend.
///
/// Accessibility permission must be granted before `start()` is called.
/// The check happens in `start()` so that `new()` always succeeds.
pub fn create_input_capture() -> Result<Box<dyn InputCapture>, PlatformError> {
    Ok(Box::new(MacOSCapture::new()))
}

// ---------------------------------------------------------------------------
// Factory: action executor
// ---------------------------------------------------------------------------

/// Returns the CGEventPost-based action executor.
pub fn create_action_executor() -> Result<Box<dyn ActionExecutor>, PlatformError> {
    Ok(Box::new(MacOSExecutor::new()))
}
