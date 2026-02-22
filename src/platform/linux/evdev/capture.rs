//! Keyboard capture via the Linux evdev interface (/dev/input/event*).
//!
//! `LinuxEvdevCapture` implements the `InputCapture` trait. `start()` enumerates
//! all keyboard devices under /dev/input/, then spawns a background thread with a
//! single-threaded tokio runtime. The runtime drives an async event loop that reads
//! from all keyboards concurrently via `futures::stream::SelectAll`.
//!
//! Required permissions: the process user must be a member of the `input` group.
//!   sudo usermod -aG input $USER   (then log out and back in)

use std::thread;
use std::thread::JoinHandle;

use evdev::{Device, InputEventKind};
use futures::StreamExt;
use futures::stream::SelectAll;
use tokio::sync::oneshot;

// `evdev::InputEvent` and `crate::platform::InputEvent` share a name; alias ours.
use crate::platform::{
    InputCapture as InputCaptureTrait, InputEvent as PlatformInputEvent,
    KeyState, Modifiers, PlatformError, WindowContext,
};
use super::super::keycodes::evdev_to_keycode;

// ---------------------------------------------------------------------------
// Public struct
// ---------------------------------------------------------------------------

/// Linux keyboard capture backend using the evdev input subsystem.
pub struct LinuxEvdevCapture {
    stop_tx: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl LinuxEvdevCapture {
    pub fn new() -> Self {
        Self { stop_tx: None, thread: None }
    }
}

impl InputCaptureTrait for LinuxEvdevCapture {
    fn start(
        &mut self,
        callback: Box<dyn Fn(PlatformInputEvent) + Send>,
    ) -> Result<(), PlatformError> {
        // Enumerate and open keyboard devices in the calling thread so errors
        // surface immediately rather than silently dying in the background.
        let keyboards = find_keyboards()?;
        log::info!("capture: found {} keyboard device(s)", keyboards.len());
        for dev in &keyboards {
            log::debug!("capture: monitoring {:?}", dev.name().unwrap_or("unnamed"));
        }

        let (stop_tx, stop_rx) = oneshot::channel();
        self.stop_tx = Some(stop_tx);

        let thread = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("capture: failed to build tokio runtime");

            if let Err(e) = rt.block_on(capture_loop(keyboards, callback, stop_rx)) {
                log::error!("capture: fatal error: {e}");
            }
        });

        self.thread = Some(thread);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        Ok(())
    }
}

impl Drop for LinuxEvdevCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

// ---------------------------------------------------------------------------
// Device enumeration
// ---------------------------------------------------------------------------

/// Finds all keyboard devices in /dev/input/.
///
/// A device is considered a keyboard if it reports support for `KEY_A`, which
/// filters out mice, joysticks, and other non-keyboard HID devices.
///
/// Returns `Err` when no keyboards are found (commonly because the process user
/// is not in the `input` group -- see module-level documentation).
fn find_keyboards() -> Result<Vec<Device>, PlatformError> {
    let keyboards: Vec<Device> = evdev::enumerate()
        .filter_map(|(_, dev)| {
            let is_keyboard = dev
                .supported_keys()
                .is_some_and(|keys| keys.contains(evdev::Key::KEY_A));
            if is_keyboard { Some(dev) } else { None }
        })
        .collect();

    if keyboards.is_empty() {
        Err(PlatformError::Unavailable(
            "No keyboard devices found in /dev/input/. \
             Ensure this user is in the 'input' group: \
             sudo usermod -aG input $USER (then log out and back in)."
                .into(),
        ))
    } else {
        Ok(keyboards)
    }
}

// ---------------------------------------------------------------------------
// Async event loop
// ---------------------------------------------------------------------------

/// Reads keyboard events from all discovered devices concurrently until stopped.
async fn capture_loop(
    keyboards: Vec<Device>,
    callback: Box<dyn Fn(PlatformInputEvent) + Send>,
    stop_rx: oneshot::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Convert each Device into a non-blocking async EventStream.
    let mut all_streams: SelectAll<evdev::EventStream> = SelectAll::new();
    for device in keyboards {
        all_streams.push(device.into_event_stream()?);
    }

    log::info!("capture: evdev capture active");

    tokio::select! {
        _ = stop_rx => {
            log::info!("capture: stop signal received");
        }
        _ = async {
            while let Some(Ok(event)) = all_streams.next().await {
                handle_evdev_event(event, &*callback);
            }
            log::info!("capture: all evdev streams ended");
        } => {}
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Event handler
// ---------------------------------------------------------------------------

/// Converts a raw evdev event into a `PlatformInputEvent` and calls `callback`.
///
/// Only key-down (value 1) and key-up (value 0) are forwarded.
/// Auto-repeat (value 2) is ignored; the rule engine handles repetition.
fn handle_evdev_event(event: evdev::InputEvent, callback: &dyn Fn(PlatformInputEvent)) {
    let InputEventKind::Key(evdev_key) = event.kind() else {
        return;
    };

    let state = match event.value() {
        1 => KeyState::Down,
        0 => KeyState::Up,
        _ => return,
    };

    match evdev_to_keycode(evdev_key.code() as u32) {
        Some(key) => {
            callback(PlatformInputEvent {
                key,
                state,
                // Modifier tracking and window context are implemented in M11.
                modifiers: Modifiers::default(),
                window: WindowContext::default(),
            });
        }
        None => {
            log::debug!("capture: unknown evdev keycode {}", evdev_key.code());
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_produces_idle_state() {
        let capture = LinuxEvdevCapture::new();
        assert!(capture.stop_tx.is_none());
        assert!(capture.thread.is_none());
    }

    #[test]
    fn stop_on_unstarted_capture_is_noop() {
        let mut capture = LinuxEvdevCapture::new();
        assert!(capture.stop().is_ok());
    }
}
