//! Display server detection for Linux.
//!
//! Determines whether the current session is Wayland or X11-only by inspecting
//! the environment variables set by the session manager.
//! The result drives which platform backend is selected at startup.
//!
//! Note: `DISPLAY` being set alongside `WAYLAND_DISPLAY` means XWayland is
//! running as a compatibility layer for legacy X11 apps. Our app connects to
//! the Wayland portal via D-Bus and is a native Wayland client regardless.

use std::env;

// ---------------------------------------------------------------------------
// Display server type
// ---------------------------------------------------------------------------

/// The active Linux display server protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    /// Wayland session. `WAYLAND_DISPLAY` is set (with or without `DISPLAY`).
    /// `DISPLAY` being present alongside it means XWayland is available for
    /// legacy apps, but this process uses the Wayland portal path.
    Wayland,
    /// Pure X11 session. Only `DISPLAY` is set; no Wayland compositor present.
    X11,
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Detects the active display server from environment variables.
///
/// Returns `None` when neither `WAYLAND_DISPLAY` nor `DISPLAY` is set,
/// which indicates the process is running outside of any graphical session.
pub fn detect_display_server() -> Option<DisplayServer> {
    let has_wayland = env::var_os("WAYLAND_DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let has_display = env::var_os("DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    classify_display(has_wayland, has_display)
}

/// Classifies the display server from boolean presence flags.
///
/// Extracted from `detect_display_server` so the classification logic
/// can be unit-tested without mutating process environment variables.
fn classify_display(has_wayland: bool, has_display: bool) -> Option<DisplayServer> {
    match (has_wayland, has_display) {
        // WAYLAND_DISPLAY present means a Wayland compositor is running.
        // DISPLAY may also be set (XWayland compat layer); that is irrelevant
        // to us since we talk to the portal via D-Bus, not via a Wayland socket.
        (true, _) => Some(DisplayServer::Wayland),
        (false, true) => Some(DisplayServer::X11),
        (false, false) => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wayland_only_detects_wayland() {
        assert_eq!(classify_display(true, false), Some(DisplayServer::Wayland));
    }

    #[test]
    fn wayland_and_display_still_detects_wayland() {
        // DISPLAY being set means XWayland compat is available for other apps;
        // we are a native Wayland client and use the Wayland path regardless.
        assert_eq!(classify_display(true, true), Some(DisplayServer::Wayland));
    }

    #[test]
    fn display_only_detects_x11() {
        assert_eq!(classify_display(false, true), Some(DisplayServer::X11));
    }

    #[test]
    fn no_vars_returns_none() {
        assert_eq!(classify_display(false, false), None);
    }

    #[test]
    fn display_server_variants_are_distinct() {
        assert_ne!(DisplayServer::Wayland, DisplayServer::X11);
    }
}
