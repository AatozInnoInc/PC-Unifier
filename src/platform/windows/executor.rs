//! Windows action executor via SendInput.
//!
//! `WindowsExecutor` implements `ActionExecutor`. Injection is synchronous:
//! `SendInput` returns after the event is queued. No background thread is
//! needed. Only `Action::InjectKey` is handled; all other variants are no-ops
//! until later milestones implement them.

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
};

use super::keycodes::keycode_to_vkcode;
use crate::platform::{Action, ActionExecutor, KeyState, PlatformError};

// ---------------------------------------------------------------------------
// Public struct
// ---------------------------------------------------------------------------

/// Injects keyboard events via SendInput on Windows.
///
/// Stateless: each `execute()` call builds an `INPUT` record and calls
/// `SendInput` synchronously. No background thread is required.
pub struct WindowsExecutor;

impl WindowsExecutor {
    pub fn new() -> Self {
        WindowsExecutor
    }
}

// ---------------------------------------------------------------------------
// ActionExecutor trait impl
// ---------------------------------------------------------------------------

impl ActionExecutor for WindowsExecutor {
    /// Executes `Action::InjectKey` by posting a `KEYBDINPUT` event.
    ///
    /// All other action variants are silently accepted and ignored until later
    /// milestones implement them.
    fn execute(&self, action: &Action) -> Result<(), PlatformError> {
        let Action::InjectKey { key, state } = action else {
            return Ok(());
        };

        let Some((vk, extra_flags)) = keycode_to_vkcode(*key) else {
            log::debug!("executor: no Windows VK code for {:?}, skipping", key);
            return Ok(());
        };

        let mut dw_flags = extra_flags;
        if *state == KeyState::Up {
            dw_flags |= KEYEVENTF_KEYUP;
        }

        let captured_at = std::time::Instant::now();

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: dw_flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let sent = unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) };

        if sent == 0 {
            return Err(PlatformError::Other("SendInput returned 0".into()));
        }

        log::debug!(
            "executor: injected {:?} {:?} in {:.2}ms",
            key,
            state,
            captured_at.elapsed().as_secs_f64() * 1000.0
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
        let executor = WindowsExecutor::new();
        assert!(executor.execute(&Action::Passthrough).is_ok());
        assert!(executor.execute(&Action::Suppress).is_ok());
        assert!(executor
            .execute(&Action::Exec {
                command: "cmd".into()
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
