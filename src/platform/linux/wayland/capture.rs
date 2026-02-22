//! Wayland input capture via xdg-desktop-portal InputCapture portal and libei (reis).
//!
//! `LinuxWaylandCapture` implements the `InputCapture` trait. `start()` spawns a
//! background thread that owns a single-threaded tokio runtime; that runtime runs
//! the portal session setup and the libei EIS event loop.  Each keyboard event is
//! converted to a canonical `InputEvent` and delivered to the caller's callback.
//!
//! The background thread exits cleanly when `stop()` sends a cancellation signal
//! via a `tokio::sync::oneshot` channel.

use std::os::unix::net::UnixStream;
use std::thread;

use ashpd::desktop::input_capture::{Capabilities, InputCapture};
use futures::StreamExt;
use reis::ei;
use reis::event::{DeviceCapability, EiEvent};
use tokio::sync::oneshot;

use super::super::keycodes::{evdev_to_keycode, key_state_from_reis};
use crate::platform::{
    InputCapture as InputCaptureTrait, InputEvent, Modifiers, PlatformError, WindowContext,
};

// ---------------------------------------------------------------------------
// Public struct
// ---------------------------------------------------------------------------

/// Captures keyboard input via xdg-desktop-portal InputCapture + libei on Wayland.
///
/// Modifiers and window context fields are left at `Default` until M11.
pub struct LinuxWaylandCapture {
    stop_tx: Option<oneshot::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl LinuxWaylandCapture {
    pub fn new() -> Self {
        Self {
            stop_tx: None,
            thread: None,
        }
    }
}

// ---------------------------------------------------------------------------
// InputCapture trait impl
// ---------------------------------------------------------------------------

impl InputCaptureTrait for LinuxWaylandCapture {
    /// Spawns a background thread that connects to the InputCapture portal and
    /// delivers keyboard events to `callback` for the lifetime of the capture.
    fn start(&mut self, callback: Box<dyn Fn(InputEvent) + Send>) -> Result<(), PlatformError> {
        let (stop_tx, stop_rx) = oneshot::channel::<()>();

        let thread = thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    log::error!("capture: failed to build tokio runtime: {e}");
                    return;
                }
            };
            rt.block_on(run_capture(callback, stop_rx));
        });

        self.stop_tx = Some(stop_tx);
        self.thread = Some(thread);
        Ok(())
    }

    /// Signals the capture thread to stop and waits for it to exit.
    fn stop(&mut self) -> Result<(), PlatformError> {
        if let Some(tx) = self.stop_tx.take() {
            // Ignore send error: the thread may have already exited.
            let _ = tx.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Async capture loop
// ---------------------------------------------------------------------------

/// Entry point for the capture background thread's async block.
/// Logs errors from `capture_loop` rather than propagating them.
async fn run_capture(callback: Box<dyn Fn(InputEvent) + Send>, stop_rx: oneshot::Receiver<()>) {
    if let Err(e) = capture_loop(callback, stop_rx).await {
        log::error!("capture: {e}");
    }
}

/// Connects to the InputCapture portal, opens the EIS socket, and drives the
/// libei event loop until a stop signal is received or the stream closes.
async fn capture_loop(
    callback: Box<dyn Fn(InputEvent) + Send>,
    stop_rx: oneshot::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the portal via D-Bus.
    let portal = InputCapture::new().await?;

    // Request a keyboard-only capture session.
    let (session, granted_caps) = portal
        .create_session(None, Capabilities::Keyboard.into())
        .await?;
    log::debug!(
        "capture: session created, granted capabilities: {:?}",
        granted_caps
    );

    // The portal protocol requires GetZones + SetPointerBarriers before Enable,
    // even for keyboard-only sessions. An empty barrier list tells the compositor
    // there are no pointer triggers; it should activate capture immediately.
    let zones = portal.zones(&session).await?.response()?;
    log::debug!(
        "capture: got {} zone(s), zone_set={}",
        zones.regions().len(),
        zones.zone_set()
    );
    let failed = portal
        .set_pointer_barriers(&session, &[], zones.zone_set())
        .await?
        .response()?;
    log::debug!(
        "capture: pointer barriers set (failed: {:?})",
        failed.failed_barriers()
    );

    let fd = portal.connect_to_eis(&session).await?;
    log::debug!("capture: EIS socket obtained");

    // Subscribe to Activated before Enable so the signal is not missed.
    let activated_stream = portal.receive_activated().await?;
    futures::pin_mut!(activated_stream);

    portal.enable(&session).await?;
    log::debug!("capture: portal enable acknowledged");

    let stream = UnixStream::from(fd);
    let context = ei::Context::new(stream)?;

    // Perform the libei protocol handshake as a Receiver (capture side).
    let (_conn, mut events) = context
        .handshake_tokio("pcunifier", ei::handshake::ContextType::Receiver)
        .await?;

    log::info!("capture: keyboard capture active");

    // Drive the EIS event stream and the D-Bus Activated signal concurrently.
    tokio::select! {
        _ = stop_rx => {
            log::info!("capture: stop signal received");
        }
        _ = async {
            while let Some(result) = events.next().await {
                match result {
                    Ok(event) => {
                        log::debug!("capture: EIS event received");
                        handle_ei_event(event, &*callback, &context);
                    }
                    Err(e) => log::warn!("capture: libei protocol error: {e}"),
                }
            }
            log::info!("capture: EIS stream ended");
        } => {}
        _ = async {
            // Log each Activated signal so we know when the compositor routes
            // keyboard events to our session.
            while let Some(activated) = activated_stream.next().await {
                log::info!(
                    "capture: portal Activated (activation_id: {:?})",
                    activated.activation_id()
                );
            }
            log::debug!("capture: Activated signal stream ended");
        } => {}
    }

    // Best-effort: disable the session so the compositor reclaims input.
    let _ = portal.disable(&session).await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Event handler
// ---------------------------------------------------------------------------

/// Processes a single libei event, calling `callback` for each keyboard key event.
fn handle_ei_event(event: EiEvent, callback: &dyn Fn(InputEvent), context: &ei::Context) {
    match event {
        EiEvent::SeatAdded(seat_evt) => {
            log::debug!("capture: SeatAdded -- binding keyboard capability");
            seat_evt
                .seat
                .bind_capabilities(DeviceCapability::Keyboard.into());
            if let Err(e) = context.flush() {
                log::warn!("capture: flush after seat bind: {e}");
            }
        }
        EiEvent::KeyboardKey(key_evt) => {
            match evdev_to_keycode(key_evt.key) {
                Some(key) => {
                    callback(InputEvent {
                        key,
                        state: key_state_from_reis(key_evt.state),
                        // Modifier tracking and window context are added in M11.
                        modifiers: Modifiers::default(),
                        window: WindowContext::default(),
                    });
                }
                None => {
                    log::debug!("capture: unknown evdev keycode {}", key_evt.key);
                }
            }
        }
        EiEvent::DeviceAdded(evt) => {
            log::debug!(
                "capture: DeviceAdded -- name={:?} type={:?}",
                evt.device.name(),
                evt.device.device_type()
            );
        }
        EiEvent::DevicePaused(evt) => {
            log::debug!(
                "capture: DevicePaused -- name={:?} serial={}",
                evt.device.name(),
                evt.serial
            );
        }
        EiEvent::DeviceResumed(evt) => {
            log::debug!(
                "capture: DeviceResumed -- name={:?} serial={}",
                evt.device.name(),
                evt.serial
            );
        }
        EiEvent::KeyboardModifiers(_) => {
            // Modifier state tracking is implemented in M11.
        }
        other => {
            log::debug!(
                "capture: unhandled EI event: {:?}",
                std::mem::discriminant(&other)
            );
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
        let capture = LinuxWaylandCapture::new();
        assert!(capture.stop_tx.is_none());
        assert!(capture.thread.is_none());
    }

    /// Stopping a capture that was never started must return Ok and not panic.
    #[test]
    fn stop_on_unstarted_capture_is_noop() {
        let mut capture = LinuxWaylandCapture::new();
        assert!(capture.stop().is_ok());
    }
}
