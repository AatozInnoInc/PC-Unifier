//! Linux platform backend.
//!
//! Capture: direct evdev (/dev/input/event*) via `LinuxEvdevCapture`.
//! Injection: xdg-desktop-portal RemoteDesktop via `LinuxWaylandExecutor`.
//!
//! Startup detection (capture has no compositor dependency; executor does):
//! 1. `WAYLAND_DISPLAY` set  → RemoteDesktop portal available, use Wayland executor
//! 2. `DISPLAY` only (X11)   → not yet supported, clear error
//! 3. Neither variable set   → no display, clear error

mod detect;
mod evdev;
mod keycodes;
mod wayland;

use evdev::LinuxEvdevCapture;
use wayland::LinuxWaylandExecutor;

use crate::platform::{ActionExecutor, InputCapture, PlatformError};
use detect::{detect_display_server, DisplayServer};

// ---------------------------------------------------------------------------
// Factory: input capture
// ---------------------------------------------------------------------------

/// Returns the evdev-based keyboard capture backend.
///
/// Requires the process user to be in the `input` group.
pub fn create_input_capture() -> Result<Box<dyn InputCapture>, PlatformError> {
    Ok(Box::new(LinuxEvdevCapture::new()))
}

// ---------------------------------------------------------------------------
// Factory: action executor
// ---------------------------------------------------------------------------

/// Returns the appropriate `ActionExecutor` for the current session.
pub fn create_action_executor() -> Result<Box<dyn ActionExecutor>, PlatformError> {
    match detect_display_server() {
        Some(DisplayServer::Wayland) => {
            LinuxWaylandExecutor::new().map(|e| Box::new(e) as Box<dyn ActionExecutor>)
        }
        Some(DisplayServer::X11) => Err(PlatformError::Unavailable(
            "Pure X11 sessions are not yet supported.".into(),
        )),
        None => Err(PlatformError::Unavailable(
            "No display server detected.".into(),
        )),
    }
}
