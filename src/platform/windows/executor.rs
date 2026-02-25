//! Windows action executor via SendInput.
//!
//! `WindowsExecutor` implements `ActionExecutor`. Injection is synchronous:
//! `SendInput` returns after the event is queued. No background thread is
//! needed.
//!
//! Each action variant is handled by a dedicated private method; `execute`
//! is a pure dispatcher.
//!
//! Hotstring character injection (Action::Hotstring) is not yet implemented on
//! Windows; the action is accepted as a no-op until M11.

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    MAPVK_VK_TO_VSC,
};

use super::keycodes::keycode_to_vkcode;
use crate::platform::{Action, ActionExecutor, KeyCode, KeyState, PlatformError};

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
    /// Dispatches an action to the appropriate handler.
    ///
    /// Each action variant is handled by a dedicated private method so that
    /// this function remains a pure dispatcher with no per-variant logic.
    fn execute(&self, action: &Action) -> Result<(), PlatformError> {
        match action {
            Action::InjectKey { key, state } => self.inject_key(*key, *state),
            Action::Hotstring {
                backspaces,
                replacement,
            } => self.expand_hotstring(*backspaces, replacement),
            Action::Exec { command } => {
                // TODO(M11): suppress modifier chord members to prevent leakage
                // to the focused application before the command is launched.
                crate::platform::spawn_command(command)
            }
            _ => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Action handlers (private)
// ---------------------------------------------------------------------------

impl WindowsExecutor {
    /// Inject a key event via SendInput.
    fn inject_key(&self, key: KeyCode, state: KeyState) -> Result<(), PlatformError> {
        let Some((vk, extra_flags)) = keycode_to_vkcode(key) else {
            log::debug!("executor: no Windows VK code for {:?}, skipping", key);
            return Ok(());
        };

        let mut dw_flags = extra_flags;
        if state == KeyState::Up {
            dw_flags |= KEYEVENTF_KEYUP;
        }

        let inject_start = std::time::Instant::now();

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) as u16 },
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
            inject_start.elapsed().as_secs_f64() * 1000.0
        );

        Ok(())
    }

    /// Hotstring expansion is not yet implemented on Windows (planned for M11).
    fn expand_hotstring(&self, backspaces: usize, replacement: &str) -> Result<(), PlatformError> {
        log::debug!(
            "executor: hotstring expansion not yet implemented on Windows \
             ({} backspace(s), {} char(s) -- no-op)",
            backspaces,
            replacement.len()
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

    /// Hotstring is a no-op on Windows until M11.
    #[test]
    fn hotstring_is_noop() {
        let executor = WindowsExecutor::new();
        assert!(executor
            .execute(&Action::Hotstring {
                backspaces: 3,
                replacement: "hello".into(),
            })
            .is_ok());
    }
}
