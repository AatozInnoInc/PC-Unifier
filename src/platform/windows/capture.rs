//! Windows keyboard capture via WH_KEYBOARD_LL (low-level keyboard hook).
//!
//! `WindowsCapture` implements `InputCapture`. `start()` spawns a background
//! thread that installs the hook and runs a `GetMessageW` loop (required for
//! low-level hooks to deliver events). `stop()` uninstalls the hook and posts
//! `WM_QUIT` to exit the message loop, then joins the thread.
//!
//! No special permissions are required on Windows for WH_KEYBOARD_LL.
//!
//! Feedback loop prevention: `SendInput` sets `LLKHF_INJECTED` on the
//! resulting event. The hook proc checks this flag and passes injected events
//! through unchanged, so only physical key events invoke the user callback.
//!
//! Suppression: returning a non-zero `LRESULT` from the hook proc (without
//! calling `CallNextHookEx`) suppresses the original physical event. The
//! executor re-injects the processed version via `SendInput`.
//!
//! Callback storage: `WH_KEYBOARD_LL` hook procs receive no `user_info`
//! pointer, so the user callback is stored in a process-global `Mutex`.
//! Only one `WindowsCapture` instance should be active at a time.

use std::sync::mpsc;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use std::ptr;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::keycodes::vkcode_to_keycode;
use crate::platform::{
    InputCapture as InputCaptureTrait, InputEvent as PlatformInputEvent, KeyState, Modifiers,
    PlatformError, WindowContext,
};

// ---------------------------------------------------------------------------
// Process-global callback storage
// ---------------------------------------------------------------------------

/// Stores the active capture callback.
///
/// `WH_KEYBOARD_LL` hook procs have no `user_info` parameter, so the callback
/// must live in a global. At most one `WindowsCapture` should be active.
static HOOK_CALLBACK: Mutex<Option<Box<dyn Fn(PlatformInputEvent) + Send>>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Public struct
// ---------------------------------------------------------------------------

/// Windows keyboard capture backend using `WH_KEYBOARD_LL`.
pub struct WindowsCapture {
    /// Handle returned by `SetWindowsHookExW`; used to unhook in `stop()`. Stored as isize for Send.
    hook: Option<isize>,
    /// Thread ID of the background message-loop thread; used for `PostThreadMessageW`.
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl WindowsCapture {
    pub fn new() -> Self {
        Self {
            hook: None,
            thread_id: 0,
            thread: None,
        }
    }
}

// ---------------------------------------------------------------------------
// InputCapture trait impl
// ---------------------------------------------------------------------------

impl InputCaptureTrait for WindowsCapture {
    fn start(
        &mut self,
        callback: Box<dyn Fn(PlatformInputEvent) + Send>,
    ) -> Result<(), PlatformError> {
        // Store callback globally before the hook is installed.
        {
            let mut guard = HOOK_CALLBACK
                .lock()
                .map_err(|_| PlatformError::Other("callback mutex poisoned".into()))?;
            *guard = Some(callback);
        }

        // Channel: background thread sends (hook_handle, thread_id) after setup. isize for Send.
        let (info_tx, info_rx) = mpsc::channel::<Result<(isize, u32), PlatformError>>();

        let thread = thread::spawn(move || {
            // Install hook on this thread; the GetMessageW loop below keeps it alive.
            let hook = unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), ptr::null_mut(), 0)
            };

            if hook.is_null() {
                let _ = info_tx.send(Err(PlatformError::Other("SetWindowsHookExW failed".into())));
                return;
            }

            let thread_id = unsafe { GetCurrentThreadId() };
            let _ = info_tx.send(Ok((hook as isize, thread_id)));

            log::info!("capture: WH_KEYBOARD_LL hook active");

            // Message loop: required for WH_KEYBOARD_LL to deliver events.
            // Returns 0 on WM_QUIT, -1 on error; both exit the loop.
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {}
            }

            log::info!("capture: message loop exited");

            unsafe { UnhookWindowsHookEx(hook) };
        });

        match info_rx.recv() {
            Ok(Ok((hook, thread_id))) => {
                self.hook = Some(hook);
                self.thread_id = thread_id;
                self.thread = Some(thread);
                Ok(())
            }
            Ok(Err(e)) => {
                // Background thread reported an error; clear callback and propagate.
                let _ = HOOK_CALLBACK.lock().map(|mut g| *g = None);
                Err(e)
            }
            Err(_) => Err(PlatformError::Other(
                "capture thread exited before reporting hook status".into(),
            )),
        }
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        // Unhook first so no further callbacks fire after this returns.
        if let Some(hook) = self.hook.take() {
            unsafe { UnhookWindowsHookEx(hook as HHOOK) };
        }

        // Clear the callback while certain no more hook_proc calls are in flight.
        let _ = HOOK_CALLBACK.lock().map(|mut g| *g = None);

        // Signal the message loop to exit.
        if self.thread_id != 0 {
            unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) };
            self.thread_id = 0;
        }

        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }

        Ok(())
    }
}

impl Drop for WindowsCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

// ---------------------------------------------------------------------------
// Hook procedure
// ---------------------------------------------------------------------------

/// Low-level keyboard hook proc, called on the background message-loop thread.
///
/// Physical events (no `LLKHF_INJECTED`): invoke the callback, suppress the
/// original event (return 1). The executor re-injects the processed version.
///
/// Injected events (`LLKHF_INJECTED`): pass through via `CallNextHookEx`
/// so the re-injected event reaches the application normally.
///
/// Unknown key codes: pass through so the user is not locked out.
unsafe extern "system" fn hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code != HC_ACTION as i32 {
        return CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param);
    }

    let kb = &*(l_param as *const KBDLLHOOKSTRUCT);

    // Pass injected events (our own SendInput) through unchanged.
    if kb.flags & LLKHF_INJECTED != 0 {
        return CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param);
    }

    let key_state = match w_param as u32 {
        WM_KEYDOWN | WM_SYSKEYDOWN => KeyState::Down,
        WM_KEYUP | WM_SYSKEYUP => KeyState::Up,
        _ => return CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param),
    };

    let extended = kb.flags & LLKHF_EXTENDED != 0;

    match vkcode_to_keycode(kb.vkCode as u16, extended) {
        Some(key) => {
            log::info!("capture: key {:?} {:?}", key, key_state);
            if let Ok(guard) = HOOK_CALLBACK.lock() {
                if let Some(cb) = guard.as_ref() {
                    cb(PlatformInputEvent {
                        key,
                        state: key_state,
                        // Modifier tracking and window context are implemented in M11.
                        modifiers: Modifiers::default(),
                        window: WindowContext::default(),
                    });
                }
            }
            // Suppress original; executor will re-inject the processed version.
            1
        }
        None => {
            log::debug!("capture: unknown VK code {:#04x}", kb.vkCode);
            // Unknown key: pass through so the user is not locked out.
            CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param)
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
        let capture = WindowsCapture::new();
        assert!(capture.hook.is_none());
        assert_eq!(capture.thread_id, 0);
        assert!(capture.thread.is_none());
    }

    /// Stopping a capture that was never started must return Ok and not panic.
    #[test]
    fn stop_on_unstarted_capture_is_noop() {
        let mut capture = WindowsCapture::new();
        assert!(capture.stop().is_ok());
    }
}
