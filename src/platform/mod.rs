//! Platform abstraction layer.
//!
//! Defines the InputCapture and ActionExecutor traits, along with all shared
//! types that platform backends must use. Platform-specific implementations
//! live in child modules.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{create_action_executor, create_input_capture};
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

// ---------------------------------------------------------------------------
// Key representation
// ---------------------------------------------------------------------------

/// Canonical key codes derived from the config schema.
///
/// Config-level aliases (Control, Super, Win, Cmd, Return) are resolved by
/// the config parser in M7. This enum contains only canonical names.
/// Platform backends normalize left/right modifier variants into the unified
/// `Ctrl`, `Shift`, `Alt`, and `Meta` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Top-row digits (prefixed to form valid identifiers)
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    // Modifiers (unified -- backends normalize left/right into these)
    Ctrl,
    Shift,
    Alt,
    Meta,

    // Navigation and editing
    Space,
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,

    // Lock and system keys
    CapsLock,
    NumLock,
    ScrollLock,
    PrintScreen,
    Pause,

    // Numeric keypad
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSub,
    NumpadMul,
    NumpadDiv,
    NumpadEnter,

    // Punctuation / symbol keys
    Backtick,
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Apostrophe,
    Comma,
    Period,
    Slash,
}

// ---------------------------------------------------------------------------
// Key state
// ---------------------------------------------------------------------------

/// Whether a key was pressed or released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyState {
    Down,
    Up,
}

// ---------------------------------------------------------------------------
// Modifiers
// ---------------------------------------------------------------------------

/// Active modifier flags at the time of an input event.
///
/// Uses plain booleans rather than bitflags to keep the dependency count at
/// zero. Platform backends normalize left/right modifier variants into these
/// unified flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

// ---------------------------------------------------------------------------
// Window context
// ---------------------------------------------------------------------------

/// Metadata about the focused window when an input event occurred.
///
/// Fields default to `None` until window-context integration in M11.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowContext {
    pub app_id: Option<String>,
    pub title: Option<String>,
}

// ---------------------------------------------------------------------------
// Input event
// ---------------------------------------------------------------------------

/// A single input event captured from the platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEvent {
    pub key: KeyCode,
    pub state: KeyState,
    pub modifiers: Modifiers,
    pub window: WindowContext,
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

/// An action the engine asks the platform backend to execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Remap one key press to another.
    Remap { from: KeyCode, to: KeyCode },
    /// Execute a shell command.
    Exec { command: String },
    /// Type a string via synthetic key events.
    TypeString { text: String },
    /// Let the original event pass through unmodified.
    Passthrough,
    /// Suppress (swallow) the original event.
    Suppress,
    /// Directly inject a key event with explicit state.
    ///
    /// Used by platform backends and the rule engine (M8) when a higher-level
    /// action (Remap, Passthrough) has been resolved to a concrete key + state
    /// pair. Backends that need the current event state (Down/Up) to inject
    /// correctly should receive this variant rather than Remap or Passthrough.
    InjectKey { key: KeyCode, state: KeyState },
}

// ---------------------------------------------------------------------------
// Platform error
// ---------------------------------------------------------------------------

/// Errors returned by platform operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlatformError {
    /// Required permission was not granted (e.g. macOS accessibility).
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Platform feature is not available (e.g. missing Wayland compositor).
    #[error("unavailable: {0}")]
    Unavailable(String),

    /// Any other platform error.
    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Captures raw input events from the platform and delivers them via callback.
///
/// Implementors must be `Send` so the capture can be moved across threads.
pub trait InputCapture: Send {
    /// Begin capturing input events, invoking `callback` for each event.
    fn start(&mut self, callback: Box<dyn Fn(InputEvent) + Send>) -> Result<(), PlatformError>;

    /// Stop capturing input events.
    fn stop(&mut self) -> Result<(), PlatformError>;
}

/// Executes actions on the platform (key synthesis, command execution, etc.).
///
/// Implementors must be `Send` so the executor can be used across threads.
pub trait ActionExecutor: Send {
    /// Execute the given action.
    fn execute(&self, action: &Action) -> Result<(), PlatformError>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_code_variants_construct() {
        // Spot-check representative variants from each category
        let _letter = KeyCode::A;
        let _digit = KeyCode::Key0;
        let _fkey = KeyCode::F24;
        let _modifier = KeyCode::Meta;
        let _nav = KeyCode::PageDown;
        let _numpad = KeyCode::NumpadEnter;
        let _punct = KeyCode::Backtick;
    }

    #[test]
    fn key_state_variants_are_distinct() {
        assert_ne!(KeyState::Down, KeyState::Up);
    }

    #[test]
    fn default_modifiers_all_false() {
        let m = Modifiers::default();
        assert!(!m.ctrl);
        assert!(!m.shift);
        assert!(!m.alt);
        assert!(!m.meta);
    }

    #[test]
    fn modifiers_field_access() {
        let m = Modifiers {
            ctrl: true,
            shift: false,
            alt: true,
            meta: false,
        };
        assert!(m.ctrl);
        assert!(!m.shift);
        assert!(m.alt);
        assert!(!m.meta);
    }

    #[test]
    fn default_window_context_all_none() {
        let wc = WindowContext::default();
        assert!(wc.app_id.is_none());
        assert!(wc.title.is_none());
    }

    #[test]
    fn window_context_field_access() {
        let wc = WindowContext {
            app_id: Some("firefox".into()),
            title: Some("Example Page".into()),
        };
        assert_eq!(wc.app_id.as_deref(), Some("firefox"));
        assert_eq!(wc.title.as_deref(), Some("Example Page"));
    }

    #[test]
    fn input_event_construction() {
        let event = InputEvent {
            key: KeyCode::A,
            state: KeyState::Down,
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
            window: WindowContext::default(),
        };
        assert_eq!(event.key, KeyCode::A);
        assert_eq!(event.state, KeyState::Down);
        assert!(event.modifiers.ctrl);
        assert!(event.window.app_id.is_none());
    }

    #[test]
    fn action_variants_construct() {
        let _remap = Action::Remap {
            from: KeyCode::A,
            to: KeyCode::B,
        };
        let _exec = Action::Exec {
            command: "echo hello".into(),
        };
        let _type_str = Action::TypeString {
            text: "hello".into(),
        };
        let _pass = Action::Passthrough;
        let _suppress = Action::Suppress;
        let _inject = Action::InjectKey {
            key: KeyCode::A,
            state: KeyState::Down,
        };
    }

    #[test]
    fn inject_key_equality() {
        let a = Action::InjectKey { key: KeyCode::Enter, state: KeyState::Up };
        let b = Action::InjectKey { key: KeyCode::Enter, state: KeyState::Up };
        let c = Action::InjectKey { key: KeyCode::Enter, state: KeyState::Down };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn platform_error_display() {
        let e = PlatformError::PermissionDenied("accessibility not granted".into());
        assert_eq!(
            e.to_string(),
            "permission denied: accessibility not granted"
        );

        let e = PlatformError::Unavailable("no compositor".into());
        assert_eq!(e.to_string(), "unavailable: no compositor");

        let e = PlatformError::Other("something went wrong".into());
        assert_eq!(e.to_string(), "something went wrong");
    }

    #[test]
    fn platform_error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(PlatformError::Other("test".into()));
        let _ = e.to_string();
    }

    /// Compile-time assertion that trait signatures are well-formed.
    /// This function is never called; it only needs to compile.
    #[allow(dead_code)]
    fn assert_trait_signatures<C: InputCapture, E: ActionExecutor>() {
        fn use_capture(mut c: impl InputCapture) {
            let _ = c.start(Box::new(|_event: InputEvent| {}));
            let _ = c.stop();
        }
        fn use_executor(e: impl ActionExecutor) {
            let _ = e.execute(&Action::Passthrough);
        }
    }
}
