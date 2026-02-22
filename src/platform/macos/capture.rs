//! macOS keyboard capture via CGEventTap and CFRunLoop.
//!
//! `MacOSCapture` implements `InputCapture`. `start()` creates the event tap
//! on the calling thread so that permission errors surface immediately, then
//! spawns a background thread that adds the tap to a CFRunLoop and drives it.
//!
//! Required permissions: Accessibility must be granted in
//!   System Settings > Privacy & Security > Accessibility.
//! `AXIsProcessTrusted()` is called first; if it returns false the call fails
//! with `PlatformError::PermissionDenied` before any tap is created.
//!
//! Memory ownership:
//!   The background thread owns the tap port (CFMachPortRef), the initial
//!   run loop source, and the callback state (TapState). All three are
//!   released after `CFRunLoopRun` returns (i.e. after `stop()` completes).
//!
//! Keycode asymmetry: F13/F14/F15 share vkcodes with PrintScreen/ScrollLock/Pause;
//! capture always yields F13/F14/F15. See `docs/platform-macos.md` for details.

use std::ffi::c_void;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

use super::keycodes::vkcode_to_keycode;
use crate::platform::{
    InputCapture as InputCaptureTrait, InputEvent as PlatformInputEvent, KeyState, Modifiers,
    PlatformError, WindowContext,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// CGEventType value for key-down events.
const CG_EVENT_KEY_DOWN: u32 = 10;

/// CGEventType value for key-up events.
const CG_EVENT_KEY_UP: u32 = 11;

/// Event mask: KeyDown | KeyUp.
/// FlagsChanged (modifier events) are deferred to M11.
const EVENT_MASK: u64 = (1u64 << CG_EVENT_KEY_DOWN) | (1u64 << CG_EVENT_KEY_UP);

/// kCGKeyboardEventKeycode: CGEventField index for the virtual key code.
const CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

/// kCGHIDEventTap: tap at the HID level, before event dispatch.
const CG_HID_EVENT_TAP: u32 = 0;

/// kCGHeadInsertEventTap: insert tap at the head of the event tap list.
const CG_HEAD_INSERT_EVENT_TAP: u32 = 0;

/// kCGEventTapOptionDefault: active tap; the callback may modify or suppress events.
const CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

// ---------------------------------------------------------------------------
// Raw FFI types and declarations
// ---------------------------------------------------------------------------

type CFMachPortRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFStringRef = *const c_void;
type CGEventRef = *mut c_void;
type CGEventTapProxy = *mut c_void;

/// Signature required by CGEventTapCreate for the C callback.
type CGEventTapCallBack = unsafe extern "C" fn(
    proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    /// Returns true if this process has been granted Accessibility permission.
    fn AXIsProcessTrusted() -> bool;

    /// Creates an event tap; returns null on permission failure or system error.
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;

    /// Enables or disables an event tap.
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);

    /// Reads an integer-valued field from a CGEvent.
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    /// Creates a CFRunLoopSource backed by a CFMachPort.
    fn CFMachPortCreateRunLoopSource(
        allocator: *mut c_void,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;

    /// Returns the CFRunLoop for the calling thread.
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;

    /// Adds a source to a run loop for the given mode.
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);

    /// Runs the current thread's run loop until CFRunLoopStop is called.
    fn CFRunLoopRun();

    /// Stops the specified run loop.
    fn CFRunLoopStop(rl: CFRunLoopRef);

    /// Releases a Core Foundation object.
    fn CFRelease(cf: *const c_void);

    /// The default run loop mode constant.
    static kCFRunLoopDefaultMode: CFStringRef;
}

// ---------------------------------------------------------------------------
// Thread-safety wrappers for raw pointers
// ---------------------------------------------------------------------------

/// Wraps CFRunLoopRef for cross-thread transfer.
///
/// Apple's documentation states that CFRunLoopStop may be called from any
/// thread. CFRunLoopRef itself follows CF thread-safety rules (safe to share).
struct SendableRunLoop(CFRunLoopRef);
unsafe impl Send for SendableRunLoop {}

/// Wraps CFMachPortRef for cross-thread transfer.
///
/// Core Foundation types are safe to share between threads per Apple docs.
struct SendableMachPort(CFMachPortRef);
unsafe impl Send for SendableMachPort {}

/// Wraps *mut TapState for cross-thread transfer.
///
/// The raw pointer is handed off to the background thread which becomes the
/// sole owner. The calling thread no longer accesses it after handoff.
struct SendableStatePtr(*mut TapState);
unsafe impl Send for SendableStatePtr {}

// ---------------------------------------------------------------------------
// Callback state
// ---------------------------------------------------------------------------

/// Heap-allocated state passed to the C callback via the `user_info` pointer.
///
/// Kept alive (via `Box::into_raw`) for the full lifetime of the event tap.
/// The background thread reclaims it with `Box::from_raw` after `CFRunLoopRun`
/// returns.
struct TapState {
    callback: Box<dyn Fn(PlatformInputEvent) + Send>,
}

// ---------------------------------------------------------------------------
// Public struct
// ---------------------------------------------------------------------------

/// macOS keyboard capture backend using CGEventTap.
pub struct MacOSCapture {
    run_loop: Option<SendableRunLoop>,
    thread: Option<JoinHandle<()>>,
}

impl MacOSCapture {
    pub fn new() -> Self {
        Self {
            run_loop: None,
            thread: None,
        }
    }
}

// ---------------------------------------------------------------------------
// InputCapture trait impl
// ---------------------------------------------------------------------------

impl InputCaptureTrait for MacOSCapture {
    fn start(
        &mut self,
        callback: Box<dyn Fn(PlatformInputEvent) + Send>,
    ) -> Result<(), PlatformError> {
        if self.run_loop.is_some() {
            return Err(PlatformError::Other("capture is already running".into()));
        }

        // Fail fast with a clear message rather than letting CGEventTapCreate
        // return null without explanation.
        if !unsafe { AXIsProcessTrusted() } {
            return Err(PlatformError::PermissionDenied(
                "Accessibility permission required. \
                 Grant it in System Settings > Privacy & Security > Accessibility."
                    .into(),
            ));
        }

        // Heap-allocate TapState so its address is stable for the tap lifetime.
        let state_ptr = Box::into_raw(Box::new(TapState { callback }));

        // Create the tap on the calling thread so errors surface synchronously.
        let tap_port = unsafe {
            CGEventTapCreate(
                CG_HID_EVENT_TAP,
                CG_HEAD_INSERT_EVENT_TAP,
                CG_EVENT_TAP_OPTION_DEFAULT,
                EVENT_MASK,
                event_tap_callback,
                state_ptr.cast::<c_void>(),
            )
        };

        if tap_port.is_null() {
            // Reclaim TapState before returning the error.
            drop(unsafe { Box::from_raw(state_ptr) });
            return Err(PlatformError::PermissionDenied(
                "CGEventTapCreate returned null. \
                 Verify Accessibility permission is active."
                    .into(),
            ));
        }

        // Send pointers into the worker via channel so the spawn closure only captures
        // Send types (the channel). The worker receives and owns them on its thread.
        let (handoff_tx, handoff_rx) = mpsc::channel::<(SendableMachPort, SendableStatePtr)>();
        let _ = handoff_tx.send((SendableMachPort(tap_port), SendableStatePtr(state_ptr)));

        // Channel to receive the background thread's run loop reference.
        let (rl_tx, rl_rx) = mpsc::channel::<SendableRunLoop>();

        let thread = thread::spawn(move || {
            let (sendable_tap, sendable_state) = match handoff_rx.recv() {
                Ok(pair) => pair,
                Err(_) => return,
            };
            let tap_port = sendable_tap.0;
            let state_ptr = sendable_state.0;

            unsafe {
                let source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), tap_port, 0);

                let run_loop = CFRunLoopGetCurrent();
                CFRunLoopAddSource(run_loop, source, kCFRunLoopDefaultMode);
                // The run loop now retains the source; release our reference.
                CFRelease(source.cast::<c_void>());

                CGEventTapEnable(tap_port, true);
                log::info!("capture: CGEventTap active");

                // Notify the calling thread that the run loop is ready.
                let _ = rl_tx.send(SendableRunLoop(run_loop));

                // Block until stop() calls CFRunLoopStop.
                CFRunLoopRun();

                log::info!("capture: CFRunLoop exited");

                // Disable the tap and release all owned resources.
                CGEventTapEnable(tap_port, false);
                CFRelease(tap_port.cast::<c_void>());
                drop(Box::from_raw(state_ptr));
            }
        });

        // Wait for the background thread to confirm the run loop is running
        // before returning, so the first event can be captured immediately.
        match rl_rx.recv() {
            Ok(rl) => {
                self.run_loop = Some(rl);
                self.thread = Some(thread);
                Ok(())
            }
            Err(_) => {
                log::warn!("capture: background thread exited before run loop was ready");
                let _ = thread.join();
                Err(PlatformError::Other(
                    "background thread exited before run loop was ready".into(),
                ))
            }
        }
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        // Signal the run loop to exit; the background thread releases the tap.
        if let Some(SendableRunLoop(rl)) = self.run_loop.take() {
            unsafe { CFRunLoopStop(rl) };
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        Ok(())
    }
}

impl Drop for MacOSCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

// ---------------------------------------------------------------------------
// C callback
// ---------------------------------------------------------------------------

/// Called by the OS on the run loop thread for each captured keyboard event.
///
/// For recognised keys the original event is suppressed (returns null) so the
/// caller's callback — and ultimately the executor — is the sole source of the
/// re-emitted event. This prevents the physical event and the injected event
/// both reaching the application (which would produce doubled keystrokes).
///
/// Unrecognised key codes and non-key event types are passed through unmodified.
unsafe extern "C" fn event_tap_callback(
    _proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    let state = &*(user_info as *const TapState);

    let key_state = match event_type {
        CG_EVENT_KEY_DOWN => KeyState::Down,
        CG_EVENT_KEY_UP => KeyState::Up,
        // Non-key events: pass through unmodified.
        _ => return event,
    };

    let vkcode = CGEventGetIntegerValueField(event, CG_KEYBOARD_EVENT_KEYCODE) as u16;

    match vkcode_to_keycode(vkcode) {
        Some(key) => {
            (state.callback)(PlatformInputEvent {
                key,
                state: key_state,
                // Modifier tracking and window context are implemented in M11.
                modifiers: Modifiers::default(),
                window: WindowContext::default(),
            });
            // Suppress the original event; the executor injects the processed
            // version at kCGSessionEventTap, downstream of this tap.
            log::debug!(
                "capture: key={:?} state={:?} modifiers={:?} window={:?}",
                key,
                key_state,
                Modifiers::default(),
                WindowContext::default()
            );
            std::ptr::null_mut()
        }
        None => {
            log::debug!("capture: unknown CGKeyCode {}", vkcode);
            // Unknown key: pass through so the user is not locked out.
            event
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
        let capture = MacOSCapture::new();
        assert!(capture.run_loop.is_none());
        assert!(capture.thread.is_none());
    }

    /// Stopping a capture that was never started must return Ok and not panic.
    #[test]
    fn stop_on_unstarted_capture_is_noop() {
        let mut capture = MacOSCapture::new();
        assert!(capture.stop().is_ok());
    }
}
