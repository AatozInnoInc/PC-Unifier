//! Wayland action executor via xdg-desktop-portal RemoteDesktop portal.
//!
//! `LinuxWaylandExecutor` implements the `ActionExecutor` trait.  `new()` spawns a
//! background thread that owns a single-threaded tokio runtime; that runtime runs
//! the portal session setup and then loops waiting for injection commands.
//!
//! `execute()` enqueues commands via a `tokio::sync::mpsc` channel using the
//! non-blocking `try_send()` so it is safe to call from both synchronous and
//! asynchronous contexts (including from within the capture callback).
//!
//! Only `Action::InjectKey` is handled here.  Other action variants are no-ops
//! until the rule engine and Lua runtime milestones are reached.

use std::path::PathBuf;
use std::thread;

use ashpd::desktop::{
    remote_desktop::{DeviceType, KeyState as PortalKeyState, RemoteDesktop},
    PersistMode,
};
use tokio::sync::mpsc;

use super::super::keycodes::keycode_to_evdev;
use crate::platform::{Action, ActionExecutor, KeyState, PlatformError};

// ---------------------------------------------------------------------------
// Internal command type
// ---------------------------------------------------------------------------

/// A single key injection command sent from `execute()` to the executor task.
struct InjectionCmd {
    /// Linux evdev keycode (same namespace as `/dev/input/`).
    keycode: i32,
    /// Key state for the injection.
    state: PortalKeyState,
    /// Timestamp captured in `execute()` to measure end-to-end injection latency.
    captured_at: std::time::Instant,
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
    cmd_tx: mpsc::Sender<InjectionCmd>,
    thread: Option<thread::JoinHandle<()>>,
}

/// Channel capacity for pending injection commands.
/// At typical typing speeds (< 20 keys/s), this will never fill.
const CMD_CAPACITY: usize = 256;

impl LinuxWaylandExecutor {
    /// Creates the executor and launches the background portal session.
    ///
    /// The portal session is established asynchronously on the background thread.
    /// The first `execute()` call may be queued before the session is ready;
    /// the executor task processes commands only after the session is established.
    pub fn new() -> Result<Self, PlatformError> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<InjectionCmd>(CMD_CAPACITY);

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
}

impl Drop for LinuxWaylandExecutor {
    fn drop(&mut self) {
        // Dropping cmd_tx closes the channel; the executor task will exit its loop.
        // The JoinHandle is dropped here as well (detaching the thread).
        drop(self.thread.take());
    }
}

// ---------------------------------------------------------------------------
// ActionExecutor trait impl
// ---------------------------------------------------------------------------

impl ActionExecutor for LinuxWaylandExecutor {
    /// Enqueues an injection command.
    ///
    /// Only `Action::InjectKey` is processed; all other variants are silently
    /// accepted (they will be handled by later milestones).
    ///
    /// Uses `try_send` (non-blocking) so it is safe to call from any context.
    fn execute(&self, action: &Action) -> Result<(), PlatformError> {
        let Action::InjectKey { key, state } = action else {
            return Ok(());
        };

        let keycode = keycode_to_evdev(*key) as i32;
        let portal_state = match state {
            KeyState::Down => PortalKeyState::Pressed,
            KeyState::Up => PortalKeyState::Released,
        };

        match self.cmd_tx.try_send(InjectionCmd {
            keycode,
            state: portal_state,
            captured_at: std::time::Instant::now(),
        }) {
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

// ---------------------------------------------------------------------------
// Async executor task
// ---------------------------------------------------------------------------

/// Runs on the background thread's tokio runtime.
/// Creates the RemoteDesktop portal session, then processes injection commands
/// until the command channel is closed (executor is dropped).
async fn run_executor(mut cmd_rx: mpsc::Receiver<InjectionCmd>) {
    if let Err(e) = executor_loop(&mut cmd_rx).await {
        log::error!("executor: {e}");
    }
}

async fn executor_loop(
    cmd_rx: &mut mpsc::Receiver<InjectionCmd>,
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
        let captured_at = cmd.captured_at;
        if let Err(e) = portal
            .notify_keyboard_keycode(&session, cmd.keycode, cmd.state)
            .await
        {
            log::warn!("executor: notify_keyboard_keycode failed: {e}");
        } else {
            log::debug!(
                "executor: injected in {:.2}ms",
                captured_at.elapsed().as_secs_f64() * 1000.0
            );
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

    /// Verifies that Action::InjectKey is the only variant that produces a command.
    /// We test with a closed channel (executor not running) to confirm behavior.
    #[test]
    fn other_actions_are_noop() {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<InjectionCmd>(1);
        let executor = LinuxWaylandExecutor {
            cmd_tx,
            thread: None,
        };

        // These should all return Ok without touching the channel.
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
        // Channel with capacity 0 can't be created with tokio mpsc, use capacity 1
        // and fill it before testing overflow.
        let (cmd_tx, _cmd_rx) = mpsc::channel::<InjectionCmd>(1);
        // Fill the channel.
        cmd_tx
            .try_send(InjectionCmd {
                keycode: 30,
                state: PortalKeyState::Pressed,
                captured_at: std::time::Instant::now(),
            })
            .unwrap();
        let executor = LinuxWaylandExecutor {
            cmd_tx,
            thread: None,
        };

        // A second send should overflow and return Ok (drop, not error).
        let result = executor.execute(&Action::InjectKey {
            key: KeyCode::A,
            state: KeyState::Down,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn inject_key_on_closed_channel_returns_error() {
        let (cmd_tx, cmd_rx) = mpsc::channel::<InjectionCmd>(1);
        drop(cmd_rx); // Close the receiving end.
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
}
