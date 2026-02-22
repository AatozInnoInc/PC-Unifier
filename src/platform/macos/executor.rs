//! macOS action executor via CGEventPost.
//!
//! `MacOSExecutor` implements `ActionExecutor`. Injection is synchronous:
//! `CGEventPost` delivers the event before returning, so no background thread
//! is needed. Only `Action::InjectKey` is handled; all other variants are
//! no-ops until later milestones implement them.

use std::ffi::c_void;

use super::keycodes::keycode_to_vkcode;
use crate::platform::{Action, ActionExecutor, KeyState, PlatformError};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// CGEventTapLocation: kCGSessionEventTap -- post downstream of our HID-level
/// capture tap. Events injected here are not re-captured by a tap placed at
/// kCGHIDEventTap, which prevents the capture→inject→capture feedback loop.
const CG_SESSION_EVENT_TAP: u32 = 1;

/// kCGEventSourceStateHIDSystemState = 1 -- use the real HID hardware state.
const CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;

// ---------------------------------------------------------------------------
// Raw FFI
// ---------------------------------------------------------------------------

type CGEventRef = *mut c_void;
type CGEventSourceRef = *mut c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventSourceCreate(state_id: i32) -> CGEventSourceRef;
    fn CGEventCreateKeyboardEvent(
        source: CGEventSourceRef,
        virtual_key: u16,
        key_down: bool,
    ) -> CGEventRef;
    fn CGEventPost(tap_location: u32, event: CGEventRef);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}

// ---------------------------------------------------------------------------
// Public struct
// ---------------------------------------------------------------------------

/// Injects keyboard events via CGEventPost on macOS.
///
/// Stateless: each `execute()` call creates a `CGEvent`, posts it, and
/// releases it immediately. No background thread is required.
pub struct MacOSExecutor;

impl MacOSExecutor {
    pub fn new() -> Self {
        MacOSExecutor
    }
}

// ---------------------------------------------------------------------------
// ActionExecutor trait impl
// ---------------------------------------------------------------------------

impl ActionExecutor for MacOSExecutor {
    /// Executes `Action::InjectKey` by posting a `CGEvent` at the HID level.
    ///
    /// All other action variants are silently accepted and ignored until later
    /// milestones implement them.
    fn execute(&self, action: &Action) -> Result<(), PlatformError> {
        let Action::InjectKey { key, state } = action else {
            return Ok(());
        };

        let Some(vkcode) = keycode_to_vkcode(*key) else {
            log::debug!("executor: no macOS key code for {:?}, skipping", key);
            return Ok(());
        };

        let key_down = *state == KeyState::Down;
        let inject_start = std::time::Instant::now();

        unsafe {
            let source = CGEventSourceCreate(CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE);
            if source.is_null() {
                return Err(PlatformError::Other(
                    "CGEventSourceCreate returned null".into(),
                ));
            }

            let event = CGEventCreateKeyboardEvent(source, vkcode, key_down);
            if event.is_null() {
                CFRelease(source.cast::<c_void>());
                return Err(PlatformError::Other(
                    "CGEventCreateKeyboardEvent returned null".into(),
                ));
            }

            CGEventPost(CG_SESSION_EVENT_TAP, event);
            CFRelease(event.cast::<c_void>());
            CFRelease(source.cast::<c_void>());
        }

        log::debug!(
            "executor: injected {:?} {:?} in {:.2}ms",
            key,
            state,
            inject_start.elapsed().as_secs_f64() * 1000.0
        );

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Action, KeyCode};

    /// Non-InjectKey variants must return Ok without touching any OS API.
    #[test]
    fn other_actions_are_noop() {
        let executor = MacOSExecutor::new();
        assert!(executor.execute(&Action::Passthrough).is_ok());
        assert!(executor.execute(&Action::Suppress).is_ok());
        assert!(executor
            .execute(&Action::Exec {
                command: "ls".into()
            })
            .is_ok());
        assert!(executor
            .execute(&Action::Remap {
                from: KeyCode::A,
                to: KeyCode::B,
            })
            .is_ok());
    }
}
