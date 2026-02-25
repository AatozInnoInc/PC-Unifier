//! Wayland action executor via xdg-desktop-portal RemoteDesktop portal.
//!
//! `LinuxWaylandExecutor` implements the `ActionExecutor` trait. `new()` spawns
//! a background thread that owns a single-threaded tokio runtime; that runtime
//! runs the portal session setup and then loops waiting for injection commands.
//!
//! `execute()` enqueues commands via a `tokio::sync::mpsc` channel using the
//! non-blocking `try_send()` so it is safe to call from both synchronous and
//! asynchronous contexts (including from within the capture callback).
//!
//! Two injection paths are supported:
//!   - `ExecutorCmd::Keycode` -- evdev keycode via `notify_keyboard_keycode`
//!     (used for `Action::InjectKey`).
//!   - `ExecutorCmd::Keysym` -- X11 keysym via `notify_keyboard_keysym`
//!     (used for `Action::Hotstring` backspaces and replacement characters).
//!
//! Keysym mapping: ASCII printable characters (0x20–0x7E) map directly to their
//! Unicode code point. Non-ASCII characters use the XKB Unicode extension
//! (0x01000000 | codepoint). The BackSpace keysym is 0xFF08.

use std::path::PathBuf;
use std::thread;

use ashpd::desktop::{
    remote_desktop::{DeviceType, KeyState as PortalKeyState, RemoteDesktop},
    PersistMode,
};
use tokio::sync::mpsc;

use super::super::keycodes::keycode_to_evdev;
use crate::platform::{Action, ActionExecutor, KeyCode, KeyState, PlatformError};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// X11 keysym for the BackSpace key (XK_BackSpace).
const KEYSYM_BACKSPACE: u32 = 0xFF08;

// ---------------------------------------------------------------------------
// Internal command type
// ---------------------------------------------------------------------------

/// A command sent from `execute()` to the background executor task.
///
/// `Keycode` is used for normal key injection (`Action::InjectKey`).
/// `Keysym` is used for hotstring replacement (backspaces + typed characters).
enum ExecutorCmd {
    /// Inject via evdev keycode (same namespace as `/dev/input/`).
    Keycode {
        keycode: i32,
        state: PortalKeyState,
        /// Captured in `execute()` to measure end-to-end injection latency.
        captured_at: std::time::Instant,
    },
    /// Inject via X11 keysym (`notify_keyboard_keysym`).
    Keysym { keysym: u32, state: PortalKeyState },
}

// ---------------------------------------------------------------------------
// Public struct
// ---------------------------------------------------------------------------

/// Injects keyboard events via xdg-desktop-portal RemoteDesktop on Wayland.
///
/// Maintains a long-lived portal session on a background thread.
/// `execute()` is non-blocking: commands are queued and processed asynchronously.
pub struct LinuxWaylandExecutor {
    /// Bounded channel to the executor task (capacity `CMD_CAPACITY`).
    cmd_tx: mpsc::Sender<ExecutorCmd>,
    thread: Option<thread::JoinHandle<()>>,
}

/// Channel capacity for pending injection commands.
///
/// Hotstring expansion sends up to `2 * (backspaces + replacement.len())`
/// commands per trigger. 512 is ample headroom for all practical replacements.
const CMD_CAPACITY: usize = 512;

impl LinuxWaylandExecutor {
    /// Creates the executor and launches the background portal session.
    ///
    /// The portal session is established asynchronously on the background thread.
    /// The first `execute()` call may be queued before the session is ready;
    /// the executor task processes commands only after the session is established.
    pub fn new() -> Result<Self, PlatformError> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<ExecutorCmd>(CMD_CAPACITY);

        let thread = thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    log::error!("executor: failed to build tokio runtime: {e}");
                    return;
                }
            };
            rt.block_on(run_executor(cmd_rx));
        });

        Ok(Self {
            cmd_tx,
            thread: Some(thread),
        })
    }

    /// Enqueue a single command, handling channel-full and channel-closed errors.
    fn send_cmd(&self, cmd: ExecutorCmd) -> Result<(), PlatformError> {
        match self.cmd_tx.try_send(cmd) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                log::warn!("executor: injection channel full, event dropped");
                Ok(())
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err(PlatformError::Other("executor session closed".into()))
            }
        }
    }
}

impl Drop for LinuxWaylandExecutor {
    fn drop(&mut self) {
        // Dropping cmd_tx closes the channel; the executor task exits its loop.
        drop(self.thread.take());
    }
}

// ---------------------------------------------------------------------------
// ActionExecutor trait impl
// ---------------------------------------------------------------------------

impl ActionExecutor for LinuxWaylandExecutor {
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
            Action::Exec { command } => crate::platform::spawn_command(command),
            _ => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Action handlers (private)
// ---------------------------------------------------------------------------

impl LinuxWaylandExecutor {
    /// Enqueue a keycode injection command (evdev namespace).
    fn inject_key(&self, key: KeyCode, state: KeyState) -> Result<(), PlatformError> {
        let keycode = keycode_to_evdev(key) as i32;
        let portal_state = match state {
            KeyState::Down => PortalKeyState::Pressed,
            KeyState::Up => PortalKeyState::Released,
        };
        self.send_cmd(ExecutorCmd::Keycode {
            keycode,
            state: portal_state,
            captured_at: std::time::Instant::now(),
        })
    }

    /// Enqueue BackSpace keysyms to erase the trigger, then character keysyms
    /// for each character in the replacement string.
    fn expand_hotstring(&self, backspaces: usize, replacement: &str) -> Result<(), PlatformError> {
        log::debug!(
            "executor: hotstring expansion -- {} backspace(s) + {} char(s)",
            backspaces,
            replacement.len()
        );

        for _ in 0..backspaces {
            self.send_cmd(ExecutorCmd::Keysym {
                keysym: KEYSYM_BACKSPACE,
                state: PortalKeyState::Pressed,
            })?;
            self.send_cmd(ExecutorCmd::Keysym {
                keysym: KEYSYM_BACKSPACE,
                state: PortalKeyState::Released,
            })?;
        }

        for c in replacement.chars() {
            let keysym = char_to_keysym(c);
            log::debug!("executor: hotstring char '{}' -> keysym {:#06x}", c, keysym);
            self.send_cmd(ExecutorCmd::Keysym {
                keysym,
                state: PortalKeyState::Pressed,
            })?;
            self.send_cmd(ExecutorCmd::Keysym {
                keysym,
                state: PortalKeyState::Released,
            })?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Keysym helpers
// ---------------------------------------------------------------------------

/// Convert a Unicode character to an X11 keysym.
///
/// ASCII printable characters (U+0020..=U+007E) map directly to their code
/// point, which matches their X11 keysym value. All other Unicode characters
/// use the XKB Unicode extension: `0x01000000 | codepoint`.
fn char_to_keysym(c: char) -> u32 {
    let cp = c as u32;
    if (0x0020..=0x007E).contains(&cp) {
        cp
    } else {
        0x01000000 | cp
    }
}

// ---------------------------------------------------------------------------
// Async executor task
// ---------------------------------------------------------------------------

/// Runs on the background thread's tokio runtime.
/// Creates the RemoteDesktop portal session, then processes injection commands
/// until the command channel is closed (executor is dropped).
async fn run_executor(mut cmd_rx: mpsc::Receiver<ExecutorCmd>) {
    if let Err(e) = executor_loop(&mut cmd_rx).await {
        log::error!("executor: {e}");
    }
}

async fn executor_loop(
    cmd_rx: &mut mpsc::Receiver<ExecutorCmd>,
) -> Result<(), Box<dyn std::error::Error>> {
    let portal = RemoteDesktop::new().await?;
    let session = portal.create_session().await?;

    // Load any previously saved restore token so the permission dialog is
    // skipped on runs after the initial grant.
    let saved_token = load_restore_token();
    portal
        .select_devices(
            &session,
            DeviceType::Keyboard.into(),
            saved_token.as_deref(),
            // ExplicitlyRevoked: the portal saves the grant indefinitely and
            // returns a restore token we can reuse on the next start.
            PersistMode::ExplicitlyRevoked,
        )
        .await?;

    let start_response = portal.start(&session, None).await?;

    // Persist the restore token so subsequent runs skip the permission dialog.
    if let Some(token) = start_response.response()?.restore_token() {
        save_restore_token(token);
    }

    log::info!("executor: RemoteDesktop session active");

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            ExecutorCmd::Keycode {
                keycode,
                state,
                captured_at,
            } => {
                if let Err(e) = portal
                    .notify_keyboard_keycode(&session, keycode, state)
                    .await
                {
                    log::warn!("executor: notify_keyboard_keycode failed: {e}");
                } else {
                    log::debug!(
                        "executor: injected keycode in {:.2}ms",
                        captured_at.elapsed().as_secs_f64() * 1000.0
                    );
                }
            }
            ExecutorCmd::Keysym { keysym, state } => {
                if let Err(e) = portal
                    .notify_keyboard_keysym(&session, keysym as i32, state)
                    .await
                {
                    log::warn!(
                        "executor: notify_keyboard_keysym {:#06x} failed: {e}",
                        keysym
                    );
                } else {
                    log::debug!("executor: injected keysym {:#06x} {:?}", keysym, state);
                }
            }
        }
    }

    log::info!("executor: command channel closed, exiting");
    Ok(())
}

// ---------------------------------------------------------------------------
// Restore token helpers
// ---------------------------------------------------------------------------

/// Returns the path used to persist the RemoteDesktop restore token.
///
/// Respects `$XDG_CONFIG_HOME`; falls back to `$HOME/.config`.
fn token_path() -> Option<PathBuf> {
    let config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = PathBuf::from(h);
                p.push(".config");
                p
            })
        })?;
    Some(config_dir.join("pc-unifier").join("remote-desktop-token"))
}

/// Reads the restore token from disk. Returns `None` if the file is absent or
/// cannot be read.
fn load_restore_token() -> Option<String> {
    let path = token_path()?;
    match std::fs::read_to_string(&path) {
        Ok(token) => {
            let trimmed = token.trim().to_owned();
            if trimmed.is_empty() {
                None
            } else {
                log::debug!("executor: loaded restore token from {}", path.display());
                Some(trimmed)
            }
        }
        Err(_) => None,
    }
}

/// Writes the restore token to disk, creating the parent directory if needed.
fn save_restore_token(token: &str) {
    let Some(path) = token_path() else { return };
    if let Some(dir) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            log::warn!(
                "executor: could not create config dir {}: {e}",
                dir.display()
            );
            return;
        }
    }
    match std::fs::write(&path, token) {
        Ok(()) => log::debug!("executor: restore token saved to {}", path.display()),
        Err(e) => log::warn!("executor: could not save restore token: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::KeyCode;

    /// Non-InjectKey variants that do not touch the channel must return Ok.
    #[test]
    fn other_actions_are_noop() {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<ExecutorCmd>(1);
        let executor = LinuxWaylandExecutor {
            cmd_tx,
            thread: None,
        };

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
                to: KeyCode::B
            })
            .is_ok());
    }

    #[test]
    fn inject_key_on_full_channel_returns_ok() {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<ExecutorCmd>(1);
        // Fill the channel before constructing the executor.
        cmd_tx
            .try_send(ExecutorCmd::Keycode {
                keycode: 30,
                state: PortalKeyState::Pressed,
                captured_at: std::time::Instant::now(),
            })
            .unwrap();
        let executor = LinuxWaylandExecutor {
            cmd_tx,
            thread: None,
        };

        // A second send overflows -- must return Ok (drop, not error).
        let result = executor.execute(&Action::InjectKey {
            key: KeyCode::A,
            state: KeyState::Down,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn inject_key_on_closed_channel_returns_error() {
        let (cmd_tx, cmd_rx) = mpsc::channel::<ExecutorCmd>(1);
        drop(cmd_rx); // close the receiving end
        let executor = LinuxWaylandExecutor {
            cmd_tx,
            thread: None,
        };

        let result = executor.execute(&Action::InjectKey {
            key: KeyCode::A,
            state: KeyState::Down,
        });
        assert!(result.is_err());
    }

    // --- char_to_keysym ---

    #[test]
    fn ascii_printable_keysym_equals_codepoint() {
        assert_eq!(char_to_keysym('a'), 0x61);
        assert_eq!(char_to_keysym('@'), 0x40);
        assert_eq!(char_to_keysym('.'), 0x2E);
        assert_eq!(char_to_keysym(' '), 0x20);
    }

    #[test]
    fn non_ascii_keysym_uses_xkb_extension() {
        // U+00E9 LATIN SMALL LETTER E WITH ACUTE
        assert_eq!(char_to_keysym('\u{00E9}'), 0x010000E9);
    }
}
