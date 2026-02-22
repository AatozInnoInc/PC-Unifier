//! Windows virtual key code (u16) <-> KeyCode mapping.
//!
//! VK codes are from the Windows SDK (winuser.h). Left/right modifier variants
//! (VK_LSHIFT, VK_RSHIFT, etc.) are both accepted in `vkcode_to_keycode`;
//! `keycode_to_vkcode` emits the left-hand variant for injection.
//!
//! `keycode_to_vkcode` returns `(vk, extra_flags)` where `extra_flags` is
//! `KEYEVENTF_EXTENDEDKEY` (0x0001) for keys that require it (navigation keys,
//! NumpadEnter, NumpadDiv) and 0 otherwise. The executor ORs in
//! `KEYEVENTF_KEYUP` (0x0002) for key-up events.

use crate::platform::KeyCode;

/// `KEYEVENTF_EXTENDEDKEY` — set in `KEYBDINPUT.dwFlags` for extended keys.
pub const EXTENDED: u32 = 0x0001;

// ---------------------------------------------------------------------------
// Capture direction
// ---------------------------------------------------------------------------

/// Converts a Windows virtual key code and extended-key flag to a `KeyCode`.
///
/// `extended` is true when bit 0 (`LLKHF_EXTENDED`) is set in the
/// `KBDLLHOOKSTRUCT.flags` field, which distinguishes e.g. NumpadEnter
/// (VK_RETURN + extended) from the main Enter (VK_RETURN, not extended).
///
/// Returns `None` for codes with no `KeyCode` equivalent.
pub fn vkcode_to_keycode(vk: u16, extended: bool) -> Option<KeyCode> {
    match vk {
        // Letters (VK_A = 0x41 .. VK_Z = 0x5A, same as ASCII uppercase)
        0x41 => Some(KeyCode::A),
        0x42 => Some(KeyCode::B),
        0x43 => Some(KeyCode::C),
        0x44 => Some(KeyCode::D),
        0x45 => Some(KeyCode::E),
        0x46 => Some(KeyCode::F),
        0x47 => Some(KeyCode::G),
        0x48 => Some(KeyCode::H),
        0x49 => Some(KeyCode::I),
        0x4A => Some(KeyCode::J),
        0x4B => Some(KeyCode::K),
        0x4C => Some(KeyCode::L),
        0x4D => Some(KeyCode::M),
        0x4E => Some(KeyCode::N),
        0x4F => Some(KeyCode::O),
        0x50 => Some(KeyCode::P),
        0x51 => Some(KeyCode::Q),
        0x52 => Some(KeyCode::R),
        0x53 => Some(KeyCode::S),
        0x54 => Some(KeyCode::T),
        0x55 => Some(KeyCode::U),
        0x56 => Some(KeyCode::V),
        0x57 => Some(KeyCode::W),
        0x58 => Some(KeyCode::X),
        0x59 => Some(KeyCode::Y),
        0x5A => Some(KeyCode::Z),

        // Top-row digits (VK_0 = 0x30 .. VK_9 = 0x39, same as ASCII)
        0x30 => Some(KeyCode::Key0),
        0x31 => Some(KeyCode::Key1),
        0x32 => Some(KeyCode::Key2),
        0x33 => Some(KeyCode::Key3),
        0x34 => Some(KeyCode::Key4),
        0x35 => Some(KeyCode::Key5),
        0x36 => Some(KeyCode::Key6),
        0x37 => Some(KeyCode::Key7),
        0x38 => Some(KeyCode::Key8),
        0x39 => Some(KeyCode::Key9),

        // Function keys
        0x70 => Some(KeyCode::F1),
        0x71 => Some(KeyCode::F2),
        0x72 => Some(KeyCode::F3),
        0x73 => Some(KeyCode::F4),
        0x74 => Some(KeyCode::F5),
        0x75 => Some(KeyCode::F6),
        0x76 => Some(KeyCode::F7),
        0x77 => Some(KeyCode::F8),
        0x78 => Some(KeyCode::F9),
        0x79 => Some(KeyCode::F10),
        0x7A => Some(KeyCode::F11),
        0x7B => Some(KeyCode::F12),
        0x7C => Some(KeyCode::F13),
        0x7D => Some(KeyCode::F14),
        0x7E => Some(KeyCode::F15),
        0x7F => Some(KeyCode::F16),
        0x80 => Some(KeyCode::F17),
        0x81 => Some(KeyCode::F18),
        0x82 => Some(KeyCode::F19),
        0x83 => Some(KeyCode::F20),
        0x84 => Some(KeyCode::F21),
        0x85 => Some(KeyCode::F22),
        0x86 => Some(KeyCode::F23),
        0x87 => Some(KeyCode::F24),

        // Modifiers -- left and right variants map to the canonical form.
        // WH_KEYBOARD_LL sends VK_LSHIFT (0xA0) / VK_RSHIFT (0xA1), etc.
        0x10 | 0xA0 | 0xA1 => Some(KeyCode::Shift),
        0x11 | 0xA2 | 0xA3 => Some(KeyCode::Ctrl),
        0x12 | 0xA4 | 0xA5 => Some(KeyCode::Alt),
        0x5B | 0x5C => Some(KeyCode::Meta), // VK_LWIN / VK_RWIN

        // Navigation and editing
        0x20 => Some(KeyCode::Space),
        // VK_RETURN with extended bit = NumpadEnter; without = main Enter.
        0x0D if extended => Some(KeyCode::NumpadEnter),
        0x0D => Some(KeyCode::Enter),
        0x09 => Some(KeyCode::Tab),
        0x1B => Some(KeyCode::Escape),
        0x08 => Some(KeyCode::Backspace),
        0x2E => Some(KeyCode::Delete),
        0x2D => Some(KeyCode::Insert),
        0x24 => Some(KeyCode::Home),
        0x23 => Some(KeyCode::End),
        0x21 => Some(KeyCode::PageUp),
        0x22 => Some(KeyCode::PageDown),
        0x26 => Some(KeyCode::Up),
        0x28 => Some(KeyCode::Down),
        0x25 => Some(KeyCode::Left),
        0x27 => Some(KeyCode::Right),

        // Lock and system keys
        0x14 => Some(KeyCode::CapsLock),
        0x90 => Some(KeyCode::NumLock),
        0x91 => Some(KeyCode::ScrollLock),
        0x2C => Some(KeyCode::PrintScreen),
        0x13 => Some(KeyCode::Pause),

        // Numeric keypad
        0x60 => Some(KeyCode::Numpad0),
        0x61 => Some(KeyCode::Numpad1),
        0x62 => Some(KeyCode::Numpad2),
        0x63 => Some(KeyCode::Numpad3),
        0x64 => Some(KeyCode::Numpad4),
        0x65 => Some(KeyCode::Numpad5),
        0x66 => Some(KeyCode::Numpad6),
        0x67 => Some(KeyCode::Numpad7),
        0x68 => Some(KeyCode::Numpad8),
        0x69 => Some(KeyCode::Numpad9),
        0x6B => Some(KeyCode::NumpadAdd),
        0x6D => Some(KeyCode::NumpadSub),
        0x6A => Some(KeyCode::NumpadMul),
        0x6F => Some(KeyCode::NumpadDiv),

        // Punctuation / symbol keys (OEM codes, ANSI layout assumed)
        0xC0 => Some(KeyCode::Backtick),
        0xBD => Some(KeyCode::Minus),
        0xBB => Some(KeyCode::Equal),
        0xDB => Some(KeyCode::LeftBracket),
        0xDD => Some(KeyCode::RightBracket),
        0xDC => Some(KeyCode::Backslash),
        0xBA => Some(KeyCode::Semicolon),
        0xDE => Some(KeyCode::Apostrophe),
        0xBC => Some(KeyCode::Comma),
        0xBE => Some(KeyCode::Period),
        0xBF => Some(KeyCode::Slash),

        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Injection direction
// ---------------------------------------------------------------------------

/// Converts a canonical `KeyCode` to a Windows virtual key code and extra
/// `dwFlags` bits for `KEYBDINPUT`.
///
/// Returns `None` only when no reasonable injection mapping exists.
/// Navigation keys carry `EXTENDED` to distinguish them from numpad keys.
/// Modifiers use the left-hand variant.
pub fn keycode_to_vkcode(key: KeyCode) -> Option<(u16, u32)> {
    let (vk, flags) = match key {
        // Letters
        KeyCode::A => (0x41, 0),
        KeyCode::B => (0x42, 0),
        KeyCode::C => (0x43, 0),
        KeyCode::D => (0x44, 0),
        KeyCode::E => (0x45, 0),
        KeyCode::F => (0x46, 0),
        KeyCode::G => (0x47, 0),
        KeyCode::H => (0x48, 0),
        KeyCode::I => (0x49, 0),
        KeyCode::J => (0x4A, 0),
        KeyCode::K => (0x4B, 0),
        KeyCode::L => (0x4C, 0),
        KeyCode::M => (0x4D, 0),
        KeyCode::N => (0x4E, 0),
        KeyCode::O => (0x4F, 0),
        KeyCode::P => (0x50, 0),
        KeyCode::Q => (0x51, 0),
        KeyCode::R => (0x52, 0),
        KeyCode::S => (0x53, 0),
        KeyCode::T => (0x54, 0),
        KeyCode::U => (0x55, 0),
        KeyCode::V => (0x56, 0),
        KeyCode::W => (0x57, 0),
        KeyCode::X => (0x58, 0),
        KeyCode::Y => (0x59, 0),
        KeyCode::Z => (0x5A, 0),

        // Top-row digits
        KeyCode::Key0 => (0x30, 0),
        KeyCode::Key1 => (0x31, 0),
        KeyCode::Key2 => (0x32, 0),
        KeyCode::Key3 => (0x33, 0),
        KeyCode::Key4 => (0x34, 0),
        KeyCode::Key5 => (0x35, 0),
        KeyCode::Key6 => (0x36, 0),
        KeyCode::Key7 => (0x37, 0),
        KeyCode::Key8 => (0x38, 0),
        KeyCode::Key9 => (0x39, 0),

        // Function keys
        KeyCode::F1 => (0x70, 0),
        KeyCode::F2 => (0x71, 0),
        KeyCode::F3 => (0x72, 0),
        KeyCode::F4 => (0x73, 0),
        KeyCode::F5 => (0x74, 0),
        KeyCode::F6 => (0x75, 0),
        KeyCode::F7 => (0x76, 0),
        KeyCode::F8 => (0x77, 0),
        KeyCode::F9 => (0x78, 0),
        KeyCode::F10 => (0x79, 0),
        KeyCode::F11 => (0x7A, 0),
        KeyCode::F12 => (0x7B, 0),
        KeyCode::F13 => (0x7C, 0),
        KeyCode::F14 => (0x7D, 0),
        KeyCode::F15 => (0x7E, 0),
        KeyCode::F16 => (0x7F, 0),
        KeyCode::F17 => (0x80, 0),
        KeyCode::F18 => (0x81, 0),
        KeyCode::F19 => (0x82, 0),
        KeyCode::F20 => (0x83, 0),
        KeyCode::F21 => (0x84, 0),
        KeyCode::F22 => (0x85, 0),
        KeyCode::F23 => (0x86, 0),
        KeyCode::F24 => (0x87, 0),

        // Modifiers: inject as left-hand variant.
        KeyCode::Shift => (0xA0, 0), // VK_LSHIFT
        KeyCode::Ctrl => (0xA2, 0),  // VK_LCONTROL
        KeyCode::Alt => (0xA4, 0),   // VK_LMENU
        KeyCode::Meta => (0x5B, 0),  // VK_LWIN

        // Navigation and editing
        // Navigation keys need EXTENDED to distinguish from numpad equivalents.
        KeyCode::Space => (0x20, 0),
        KeyCode::Enter => (0x0D, 0),
        KeyCode::NumpadEnter => (0x0D, EXTENDED),
        KeyCode::Tab => (0x09, 0),
        KeyCode::Escape => (0x1B, 0),
        KeyCode::Backspace => (0x08, 0),
        KeyCode::Delete => (0x2E, EXTENDED),
        KeyCode::Insert => (0x2D, EXTENDED),
        KeyCode::Home => (0x24, EXTENDED),
        KeyCode::End => (0x23, EXTENDED),
        KeyCode::PageUp => (0x21, EXTENDED),
        KeyCode::PageDown => (0x22, EXTENDED),
        KeyCode::Up => (0x26, EXTENDED),
        KeyCode::Down => (0x28, EXTENDED),
        KeyCode::Left => (0x25, EXTENDED),
        KeyCode::Right => (0x27, EXTENDED),

        // Lock and system keys
        KeyCode::CapsLock => (0x14, 0),
        KeyCode::NumLock => (0x90, 0),
        KeyCode::ScrollLock => (0x91, 0),
        KeyCode::PrintScreen => (0x2C, 0),
        KeyCode::Pause => (0x13, 0),

        // Numeric keypad
        KeyCode::Numpad0 => (0x60, 0),
        KeyCode::Numpad1 => (0x61, 0),
        KeyCode::Numpad2 => (0x62, 0),
        KeyCode::Numpad3 => (0x63, 0),
        KeyCode::Numpad4 => (0x64, 0),
        KeyCode::Numpad5 => (0x65, 0),
        KeyCode::Numpad6 => (0x66, 0),
        KeyCode::Numpad7 => (0x67, 0),
        KeyCode::Numpad8 => (0x68, 0),
        KeyCode::Numpad9 => (0x69, 0),
        KeyCode::NumpadAdd => (0x6B, 0),
        KeyCode::NumpadSub => (0x6D, 0),
        KeyCode::NumpadMul => (0x6A, 0),
        KeyCode::NumpadDiv => (0x6F, EXTENDED),

        // Punctuation / symbol keys
        KeyCode::Backtick => (0xC0, 0),
        KeyCode::Minus => (0xBD, 0),
        KeyCode::Equal => (0xBB, 0),
        KeyCode::LeftBracket => (0xDB, 0),
        KeyCode::RightBracket => (0xDD, 0),
        KeyCode::Backslash => (0xDC, 0),
        KeyCode::Semicolon => (0xBA, 0),
        KeyCode::Apostrophe => (0xDE, 0),
        KeyCode::Comma => (0xBC, 0),
        KeyCode::Period => (0xBE, 0),
        KeyCode::Slash => (0xBF, 0),
    };
    Some((vk, flags))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::KeyCode;

    #[test]
    fn spot_check_letter_codes() {
        assert_eq!(vkcode_to_keycode(0x41, false), Some(KeyCode::A));
        assert_eq!(vkcode_to_keycode(0x5A, false), Some(KeyCode::Z));
    }

    #[test]
    fn spot_check_digit_codes() {
        assert_eq!(vkcode_to_keycode(0x30, false), Some(KeyCode::Key0));
        assert_eq!(vkcode_to_keycode(0x39, false), Some(KeyCode::Key9));
    }

    #[test]
    fn spot_check_function_key_codes() {
        assert_eq!(vkcode_to_keycode(0x70, false), Some(KeyCode::F1));
        assert_eq!(vkcode_to_keycode(0x7B, false), Some(KeyCode::F12));
        assert_eq!(vkcode_to_keycode(0x87, false), Some(KeyCode::F24));
    }

    #[test]
    fn numpad_enter_requires_extended_bit() {
        assert_eq!(vkcode_to_keycode(0x0D, false), Some(KeyCode::Enter));
        assert_eq!(vkcode_to_keycode(0x0D, true), Some(KeyCode::NumpadEnter));
    }

    #[test]
    fn right_modifiers_map_to_canonical() {
        assert_eq!(vkcode_to_keycode(0xA0, false), Some(KeyCode::Shift));
        assert_eq!(vkcode_to_keycode(0xA1, false), Some(KeyCode::Shift));
        assert_eq!(vkcode_to_keycode(0xA2, false), Some(KeyCode::Ctrl));
        assert_eq!(vkcode_to_keycode(0xA3, false), Some(KeyCode::Ctrl));
        assert_eq!(vkcode_to_keycode(0xA4, false), Some(KeyCode::Alt));
        assert_eq!(vkcode_to_keycode(0xA5, false), Some(KeyCode::Alt));
        assert_eq!(vkcode_to_keycode(0x5B, false), Some(KeyCode::Meta));
        assert_eq!(vkcode_to_keycode(0x5C, false), Some(KeyCode::Meta));
    }

    #[test]
    fn unknown_vkcode_returns_none() {
        assert_eq!(vkcode_to_keycode(0xFF, false), None);
    }

    #[test]
    fn round_trip_primary_mappings() {
        let cases: &[(KeyCode, u16)] = &[
            (KeyCode::A, 0x41),
            (KeyCode::Z, 0x5A),
            (KeyCode::Key0, 0x30),
            (KeyCode::F1, 0x70),
            (KeyCode::F24, 0x87),
            (KeyCode::Enter, 0x0D),
            (KeyCode::Space, 0x20),
            (KeyCode::Escape, 0x1B),
            (KeyCode::Backtick, 0xC0),
        ];
        for &(key, expected_vk) in cases {
            let (vk, _) = keycode_to_vkcode(key).expect("expected a mapping");
            assert_eq!(vk, expected_vk, "{key:?} -> vk");
            assert_eq!(
                vkcode_to_keycode(vk, false),
                Some(key),
                "vk {expected_vk:#04x} -> keycode"
            );
        }
    }

    #[test]
    fn navigation_keys_carry_extended_flag() {
        for key in [
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::PageUp,
            KeyCode::PageDown,
            KeyCode::Insert,
            KeyCode::Delete,
        ] {
            let (_, flags) = keycode_to_vkcode(key).expect("expected a mapping");
            assert_eq!(flags, EXTENDED, "{key:?} should carry EXTENDED flag");
        }
    }
}
