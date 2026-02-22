//! Linux evdev keycode conversions.
//!
//! Maps Linux input event key codes (from `linux/input-event-codes.h`, delivered
//! by libei/reis as `u32`) to and from the canonical `KeyCode` enum used
//! throughout PC Unifier.
//!
//! - `evdev_to_keycode`: for incoming capture events (may be unknown → `None`).
//! - `keycode_to_evdev`: for outgoing injection (always resolves; unified
//!   modifiers map to their left-side variant).

use crate::platform::{KeyCode, KeyState};
use reis::ei::keyboard::KeyState as EiKeyState;

// ---------------------------------------------------------------------------
// Capture side: evdev code → KeyCode
// ---------------------------------------------------------------------------

/// Converts a Linux evdev keycode to the canonical `KeyCode`.
///
/// Returns `None` for keycodes that have no corresponding `KeyCode` variant
/// (e.g. media keys, browser buttons, or hardware-specific keys). Callers
/// should log unknown codes at `debug` level and silently drop the event.
///
/// Both left and right modifier variants are unified into the single canonical
/// variant (`Ctrl`, `Shift`, `Alt`, `Meta`), matching the platform trait contract.
pub fn evdev_to_keycode(code: u32) -> Option<KeyCode> {
    match code {
        // Letters
        30 => Some(KeyCode::A),
        48 => Some(KeyCode::B),
        46 => Some(KeyCode::C),
        32 => Some(KeyCode::D),
        18 => Some(KeyCode::E),
        33 => Some(KeyCode::F),
        34 => Some(KeyCode::G),
        35 => Some(KeyCode::H),
        23 => Some(KeyCode::I),
        36 => Some(KeyCode::J),
        37 => Some(KeyCode::K),
        38 => Some(KeyCode::L),
        50 => Some(KeyCode::M),
        49 => Some(KeyCode::N),
        24 => Some(KeyCode::O),
        25 => Some(KeyCode::P),
        16 => Some(KeyCode::Q),
        19 => Some(KeyCode::R),
        31 => Some(KeyCode::S),
        20 => Some(KeyCode::T),
        22 => Some(KeyCode::U),
        47 => Some(KeyCode::V),
        17 => Some(KeyCode::W),
        45 => Some(KeyCode::X),
        21 => Some(KeyCode::Y),
        44 => Some(KeyCode::Z),

        // Top-row digits (2–11 = 1–0)
        2 => Some(KeyCode::Key1),
        3 => Some(KeyCode::Key2),
        4 => Some(KeyCode::Key3),
        5 => Some(KeyCode::Key4),
        6 => Some(KeyCode::Key5),
        7 => Some(KeyCode::Key6),
        8 => Some(KeyCode::Key7),
        9 => Some(KeyCode::Key8),
        10 => Some(KeyCode::Key9),
        11 => Some(KeyCode::Key0),

        // Function keys F1–F12
        59 => Some(KeyCode::F1),
        60 => Some(KeyCode::F2),
        61 => Some(KeyCode::F3),
        62 => Some(KeyCode::F4),
        63 => Some(KeyCode::F5),
        64 => Some(KeyCode::F6),
        65 => Some(KeyCode::F7),
        66 => Some(KeyCode::F8),
        67 => Some(KeyCode::F9),
        68 => Some(KeyCode::F10),
        87 => Some(KeyCode::F11),
        88 => Some(KeyCode::F12),

        // Function keys F13–F24
        183 => Some(KeyCode::F13),
        184 => Some(KeyCode::F14),
        185 => Some(KeyCode::F15),
        186 => Some(KeyCode::F16),
        187 => Some(KeyCode::F17),
        188 => Some(KeyCode::F18),
        189 => Some(KeyCode::F19),
        190 => Some(KeyCode::F20),
        191 => Some(KeyCode::F21),
        192 => Some(KeyCode::F22),
        193 => Some(KeyCode::F23),
        194 => Some(KeyCode::F24),

        // Modifiers: left and right both map to the unified variant.
        29 | 97 => Some(KeyCode::Ctrl),
        42 | 54 => Some(KeyCode::Shift),
        56 | 100 => Some(KeyCode::Alt),
        125 | 126 => Some(KeyCode::Meta),

        // Navigation and editing
        57 => Some(KeyCode::Space),
        28 => Some(KeyCode::Enter),
        15 => Some(KeyCode::Tab),
        1 => Some(KeyCode::Escape),
        14 => Some(KeyCode::Backspace),
        111 => Some(KeyCode::Delete),
        110 => Some(KeyCode::Insert),
        102 => Some(KeyCode::Home),
        107 => Some(KeyCode::End),
        104 => Some(KeyCode::PageUp),
        109 => Some(KeyCode::PageDown),
        103 => Some(KeyCode::Up),
        108 => Some(KeyCode::Down),
        105 => Some(KeyCode::Left),
        106 => Some(KeyCode::Right),

        // Lock and system keys
        58 => Some(KeyCode::CapsLock),
        69 => Some(KeyCode::NumLock),
        70 => Some(KeyCode::ScrollLock),
        99 => Some(KeyCode::PrintScreen),
        119 => Some(KeyCode::Pause),

        // Numeric keypad
        82 => Some(KeyCode::Numpad0),
        79 => Some(KeyCode::Numpad1),
        80 => Some(KeyCode::Numpad2),
        81 => Some(KeyCode::Numpad3),
        75 => Some(KeyCode::Numpad4),
        76 => Some(KeyCode::Numpad5),
        77 => Some(KeyCode::Numpad6),
        71 => Some(KeyCode::Numpad7),
        72 => Some(KeyCode::Numpad8),
        73 => Some(KeyCode::Numpad9),
        78 => Some(KeyCode::NumpadAdd),
        74 => Some(KeyCode::NumpadSub),
        55 => Some(KeyCode::NumpadMul),
        98 => Some(KeyCode::NumpadDiv),
        96 => Some(KeyCode::NumpadEnter),

        // Punctuation / symbol keys
        41 => Some(KeyCode::Backtick),
        12 => Some(KeyCode::Minus),
        13 => Some(KeyCode::Equal),
        26 => Some(KeyCode::LeftBracket),
        27 => Some(KeyCode::RightBracket),
        43 => Some(KeyCode::Backslash),
        39 => Some(KeyCode::Semicolon),
        40 => Some(KeyCode::Apostrophe),
        51 => Some(KeyCode::Comma),
        52 => Some(KeyCode::Period),
        53 => Some(KeyCode::Slash),

        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Emulation side: KeyCode → evdev code
// ---------------------------------------------------------------------------

/// Converts a canonical `KeyCode` to a Linux evdev keycode for injection.
///
/// Unified modifier variants map to their left-side physical key (the
/// conventional default for synthetic events).
pub fn keycode_to_evdev(key: KeyCode) -> u32 {
    match key {
        // Letters
        KeyCode::A => 30,
        KeyCode::B => 48,
        KeyCode::C => 46,
        KeyCode::D => 32,
        KeyCode::E => 18,
        KeyCode::F => 33,
        KeyCode::G => 34,
        KeyCode::H => 35,
        KeyCode::I => 23,
        KeyCode::J => 36,
        KeyCode::K => 37,
        KeyCode::L => 38,
        KeyCode::M => 50,
        KeyCode::N => 49,
        KeyCode::O => 24,
        KeyCode::P => 25,
        KeyCode::Q => 16,
        KeyCode::R => 19,
        KeyCode::S => 31,
        KeyCode::T => 20,
        KeyCode::U => 22,
        KeyCode::V => 47,
        KeyCode::W => 17,
        KeyCode::X => 45,
        KeyCode::Y => 21,
        KeyCode::Z => 44,

        // Top-row digits
        KeyCode::Key1 => 2,
        KeyCode::Key2 => 3,
        KeyCode::Key3 => 4,
        KeyCode::Key4 => 5,
        KeyCode::Key5 => 6,
        KeyCode::Key6 => 7,
        KeyCode::Key7 => 8,
        KeyCode::Key8 => 9,
        KeyCode::Key9 => 10,
        KeyCode::Key0 => 11,

        // Function keys F1–F12
        KeyCode::F1 => 59,
        KeyCode::F2 => 60,
        KeyCode::F3 => 61,
        KeyCode::F4 => 62,
        KeyCode::F5 => 63,
        KeyCode::F6 => 64,
        KeyCode::F7 => 65,
        KeyCode::F8 => 66,
        KeyCode::F9 => 67,
        KeyCode::F10 => 68,
        KeyCode::F11 => 87,
        KeyCode::F12 => 88,

        // Function keys F13–F24
        KeyCode::F13 => 183,
        KeyCode::F14 => 184,
        KeyCode::F15 => 185,
        KeyCode::F16 => 186,
        KeyCode::F17 => 187,
        KeyCode::F18 => 188,
        KeyCode::F19 => 189,
        KeyCode::F20 => 190,
        KeyCode::F21 => 191,
        KeyCode::F22 => 192,
        KeyCode::F23 => 193,
        KeyCode::F24 => 194,

        // Modifiers: emit left-side variant for synthetic events.
        KeyCode::Ctrl => 29,
        KeyCode::Shift => 42,
        KeyCode::Alt => 56,
        KeyCode::Meta => 125,

        // Navigation and editing
        KeyCode::Space => 57,
        KeyCode::Enter => 28,
        KeyCode::Tab => 15,
        KeyCode::Escape => 1,
        KeyCode::Backspace => 14,
        KeyCode::Delete => 111,
        KeyCode::Insert => 110,
        KeyCode::Home => 102,
        KeyCode::End => 107,
        KeyCode::PageUp => 104,
        KeyCode::PageDown => 109,
        KeyCode::Up => 103,
        KeyCode::Down => 108,
        KeyCode::Left => 105,
        KeyCode::Right => 106,

        // Lock and system keys
        KeyCode::CapsLock => 58,
        KeyCode::NumLock => 69,
        KeyCode::ScrollLock => 70,
        KeyCode::PrintScreen => 99,
        KeyCode::Pause => 119,

        // Numeric keypad
        KeyCode::Numpad0 => 82,
        KeyCode::Numpad1 => 79,
        KeyCode::Numpad2 => 80,
        KeyCode::Numpad3 => 81,
        KeyCode::Numpad4 => 75,
        KeyCode::Numpad5 => 76,
        KeyCode::Numpad6 => 77,
        KeyCode::Numpad7 => 71,
        KeyCode::Numpad8 => 72,
        KeyCode::Numpad9 => 73,
        KeyCode::NumpadAdd => 78,
        KeyCode::NumpadSub => 74,
        KeyCode::NumpadMul => 55,
        KeyCode::NumpadDiv => 98,
        KeyCode::NumpadEnter => 96,

        // Punctuation / symbol keys
        KeyCode::Backtick => 41,
        KeyCode::Minus => 12,
        KeyCode::Equal => 13,
        KeyCode::LeftBracket => 26,
        KeyCode::RightBracket => 27,
        KeyCode::Backslash => 43,
        KeyCode::Semicolon => 39,
        KeyCode::Apostrophe => 40,
        KeyCode::Comma => 51,
        KeyCode::Period => 52,
        KeyCode::Slash => 53,
    }
}

// ---------------------------------------------------------------------------
// Key state conversion
// ---------------------------------------------------------------------------

/// Converts a libei key state to the canonical `KeyState`.
pub fn key_state_from_reis(state: EiKeyState) -> KeyState {
    match state {
        EiKeyState::Press => KeyState::Down,
        EiKeyState::Released => KeyState::Up,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Every KeyCode has a known evdev code, and that code round-trips back
    /// through evdev_to_keycode. Unified modifier variants (Ctrl, Shift, etc.)
    /// map to the left-side code, which maps back to the same unified variant.
    #[test]
    fn round_trip_all_keycodes() {
        let all_keys = [
            KeyCode::A,
            KeyCode::B,
            KeyCode::C,
            KeyCode::D,
            KeyCode::E,
            KeyCode::F,
            KeyCode::G,
            KeyCode::H,
            KeyCode::I,
            KeyCode::J,
            KeyCode::K,
            KeyCode::L,
            KeyCode::M,
            KeyCode::N,
            KeyCode::O,
            KeyCode::P,
            KeyCode::Q,
            KeyCode::R,
            KeyCode::S,
            KeyCode::T,
            KeyCode::U,
            KeyCode::V,
            KeyCode::W,
            KeyCode::X,
            KeyCode::Y,
            KeyCode::Z,
            KeyCode::Key0,
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
            KeyCode::Key9,
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
            KeyCode::F6,
            KeyCode::F7,
            KeyCode::F8,
            KeyCode::F9,
            KeyCode::F10,
            KeyCode::F11,
            KeyCode::F12,
            KeyCode::F13,
            KeyCode::F14,
            KeyCode::F15,
            KeyCode::F16,
            KeyCode::F17,
            KeyCode::F18,
            KeyCode::F19,
            KeyCode::F20,
            KeyCode::F21,
            KeyCode::F22,
            KeyCode::F23,
            KeyCode::F24,
            KeyCode::Ctrl,
            KeyCode::Shift,
            KeyCode::Alt,
            KeyCode::Meta,
            KeyCode::Space,
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::Escape,
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::Insert,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::PageUp,
            KeyCode::PageDown,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::CapsLock,
            KeyCode::NumLock,
            KeyCode::ScrollLock,
            KeyCode::PrintScreen,
            KeyCode::Pause,
            KeyCode::Numpad0,
            KeyCode::Numpad1,
            KeyCode::Numpad2,
            KeyCode::Numpad3,
            KeyCode::Numpad4,
            KeyCode::Numpad5,
            KeyCode::Numpad6,
            KeyCode::Numpad7,
            KeyCode::Numpad8,
            KeyCode::Numpad9,
            KeyCode::NumpadAdd,
            KeyCode::NumpadSub,
            KeyCode::NumpadMul,
            KeyCode::NumpadDiv,
            KeyCode::NumpadEnter,
            KeyCode::Backtick,
            KeyCode::Minus,
            KeyCode::Equal,
            KeyCode::LeftBracket,
            KeyCode::RightBracket,
            KeyCode::Backslash,
            KeyCode::Semicolon,
            KeyCode::Apostrophe,
            KeyCode::Comma,
            KeyCode::Period,
            KeyCode::Slash,
        ];

        for key in all_keys {
            let evdev = keycode_to_evdev(key);
            let back = evdev_to_keycode(evdev);
            assert_eq!(
                back,
                Some(key),
                "round-trip failed for {key:?}: evdev={evdev}, got {back:?}"
            );
        }
    }

    #[test]
    fn right_ctrl_maps_to_ctrl() {
        assert_eq!(evdev_to_keycode(97), Some(KeyCode::Ctrl));
    }

    #[test]
    fn right_shift_maps_to_shift() {
        assert_eq!(evdev_to_keycode(54), Some(KeyCode::Shift));
    }

    #[test]
    fn right_alt_maps_to_alt() {
        assert_eq!(evdev_to_keycode(100), Some(KeyCode::Alt));
    }

    #[test]
    fn right_meta_maps_to_meta() {
        assert_eq!(evdev_to_keycode(126), Some(KeyCode::Meta));
    }

    #[test]
    fn unknown_evdev_code_returns_none() {
        // 0 is reserved / unassigned in evdev
        assert_eq!(evdev_to_keycode(0), None);
        // 999 is beyond any defined key
        assert_eq!(evdev_to_keycode(999), None);
    }

    #[test]
    fn key_state_press_maps_to_down() {
        assert_eq!(key_state_from_reis(EiKeyState::Press), KeyState::Down);
    }

    #[test]
    fn key_state_released_maps_to_up() {
        assert_eq!(key_state_from_reis(EiKeyState::Released), KeyState::Up);
    }

    #[test]
    fn spot_check_letter_codes() {
        assert_eq!(keycode_to_evdev(KeyCode::A), 30);
        assert_eq!(keycode_to_evdev(KeyCode::Z), 44);
        assert_eq!(keycode_to_evdev(KeyCode::Q), 16);
        assert_eq!(keycode_to_evdev(KeyCode::M), 50);
    }

    #[test]
    fn spot_check_digit_codes() {
        assert_eq!(keycode_to_evdev(KeyCode::Key1), 2);
        assert_eq!(keycode_to_evdev(KeyCode::Key0), 11);
    }

    #[test]
    fn spot_check_function_key_codes() {
        assert_eq!(keycode_to_evdev(KeyCode::F1), 59);
        assert_eq!(keycode_to_evdev(KeyCode::F12), 88);
        assert_eq!(keycode_to_evdev(KeyCode::F13), 183);
        assert_eq!(keycode_to_evdev(KeyCode::F24), 194);
    }
}
