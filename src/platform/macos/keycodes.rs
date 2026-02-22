//! macOS virtual key code (CGKeyCode, u16) <-> KeyCode mapping.
//!
//! Key codes are physical key positions per Apple HIToolbox/Events.h.
//! They are layout-independent: this mapping assumes an ANSI keyboard.
//!
//! Left/right modifier variants both map to the canonical KeyCode in
//! `vkcode_to_keycode`. `keycode_to_vkcode` emits the left variant for
//! injection (consistent with how system shortcuts are defined).

use crate::platform::KeyCode;

/// Converts a macOS CGKeyCode to a canonical `KeyCode`.
///
/// Returns `None` for unmapped codes (media keys, numpad decimal, etc.).
pub fn vkcode_to_keycode(vk: u16) -> Option<KeyCode> {
    match vk {
        // Letters
        0x00 => Some(KeyCode::A),
        0x0B => Some(KeyCode::B),
        0x08 => Some(KeyCode::C),
        0x02 => Some(KeyCode::D),
        0x0E => Some(KeyCode::E),
        0x03 => Some(KeyCode::F),
        0x05 => Some(KeyCode::G),
        0x04 => Some(KeyCode::H),
        0x22 => Some(KeyCode::I),
        0x26 => Some(KeyCode::J),
        0x28 => Some(KeyCode::K),
        0x25 => Some(KeyCode::L),
        0x2E => Some(KeyCode::M),
        0x2D => Some(KeyCode::N),
        0x1F => Some(KeyCode::O),
        0x23 => Some(KeyCode::P),
        0x0C => Some(KeyCode::Q),
        0x0F => Some(KeyCode::R),
        0x01 => Some(KeyCode::S),
        0x11 => Some(KeyCode::T),
        0x20 => Some(KeyCode::U),
        0x09 => Some(KeyCode::V),
        0x0D => Some(KeyCode::W),
        0x07 => Some(KeyCode::X),
        0x10 => Some(KeyCode::Y),
        0x06 => Some(KeyCode::Z),

        // Top-row digits
        0x1D => Some(KeyCode::Key0),
        0x12 => Some(KeyCode::Key1),
        0x13 => Some(KeyCode::Key2),
        0x14 => Some(KeyCode::Key3),
        0x15 => Some(KeyCode::Key4),
        0x17 => Some(KeyCode::Key5),
        0x16 => Some(KeyCode::Key6),
        0x1A => Some(KeyCode::Key7),
        0x1C => Some(KeyCode::Key8),
        0x19 => Some(KeyCode::Key9),

        // Function keys
        0x7A => Some(KeyCode::F1),
        0x78 => Some(KeyCode::F2),
        0x63 => Some(KeyCode::F3),
        0x76 => Some(KeyCode::F4),
        0x60 => Some(KeyCode::F5),
        0x61 => Some(KeyCode::F6),
        0x62 => Some(KeyCode::F7),
        0x64 => Some(KeyCode::F8),
        0x65 => Some(KeyCode::F9),
        0x6D => Some(KeyCode::F10),
        0x67 => Some(KeyCode::F11),
        0x6F => Some(KeyCode::F12),
        // F13-F15 double as PrintScreen/ScrollLock/Pause on extended keyboards.
        0x69 => Some(KeyCode::F13),
        0x6B => Some(KeyCode::F14),
        0x71 => Some(KeyCode::F15),
        0x6A => Some(KeyCode::F16),
        0x40 => Some(KeyCode::F17),
        0x4F => Some(KeyCode::F18),
        0x50 => Some(KeyCode::F19),
        0x5A => Some(KeyCode::F20),
        // F21-F24 have no standard macOS virtual key codes.

        // Modifiers: left and right variants both map to the canonical form.
        0x3B | 0x3E => Some(KeyCode::Ctrl),
        0x38 | 0x3C => Some(KeyCode::Shift),
        0x3A | 0x3D => Some(KeyCode::Alt),
        0x37 | 0x36 => Some(KeyCode::Meta),

        // Navigation and editing
        0x31 => Some(KeyCode::Space),
        0x24 => Some(KeyCode::Enter),
        0x30 => Some(KeyCode::Tab),
        0x35 => Some(KeyCode::Escape),
        0x33 => Some(KeyCode::Backspace), // kVK_Delete  (Backspace on PC)
        0x75 => Some(KeyCode::Delete),    // kVK_ForwardDelete
        0x72 => Some(KeyCode::Insert),    // kVK_Help (Insert on PC keyboards)
        0x73 => Some(KeyCode::Home),
        0x77 => Some(KeyCode::End),
        0x74 => Some(KeyCode::PageUp),
        0x79 => Some(KeyCode::PageDown),
        0x7E => Some(KeyCode::Up),
        0x7D => Some(KeyCode::Down),
        0x7B => Some(KeyCode::Left),
        0x7C => Some(KeyCode::Right),

        // Lock keys
        0x39 => Some(KeyCode::CapsLock),
        // kVK_ANSI_KeypadClear (0x47) acts as NumLock on PC-layout keyboards.
        0x47 => Some(KeyCode::NumLock),

        // Numeric keypad
        0x52 => Some(KeyCode::Numpad0),
        0x53 => Some(KeyCode::Numpad1),
        0x54 => Some(KeyCode::Numpad2),
        0x55 => Some(KeyCode::Numpad3),
        0x56 => Some(KeyCode::Numpad4),
        0x57 => Some(KeyCode::Numpad5),
        0x58 => Some(KeyCode::Numpad6),
        0x59 => Some(KeyCode::Numpad7),
        0x5B => Some(KeyCode::Numpad8),
        0x5C => Some(KeyCode::Numpad9),
        0x45 => Some(KeyCode::NumpadAdd),
        0x4E => Some(KeyCode::NumpadSub),
        0x43 => Some(KeyCode::NumpadMul),
        0x4B => Some(KeyCode::NumpadDiv),
        0x4C => Some(KeyCode::NumpadEnter),
        // kVK_ANSI_KeypadDecimal (0x41) has no KeyCode equivalent.

        // Punctuation / symbol keys
        0x32 => Some(KeyCode::Backtick),
        0x1B => Some(KeyCode::Minus),
        0x18 => Some(KeyCode::Equal),
        0x21 => Some(KeyCode::LeftBracket),
        0x1E => Some(KeyCode::RightBracket),
        0x2A => Some(KeyCode::Backslash),
        0x29 => Some(KeyCode::Semicolon),
        0x27 => Some(KeyCode::Apostrophe),
        0x2B => Some(KeyCode::Comma),
        0x2F => Some(KeyCode::Period),
        0x2C => Some(KeyCode::Slash),

        _ => None,
    }
}

/// Converts a canonical `KeyCode` to a macOS CGKeyCode.
///
/// Returns `None` for keys with no standard macOS virtual key code (F21-F24).
/// Modifier keys use the left-hand variant for injection.
/// `PrintScreen`, `ScrollLock`, and `Pause` are mapped to F13, F14, and F15
/// respectively, which is the standard macOS extended-keyboard convention.
pub fn keycode_to_vkcode(key: KeyCode) -> Option<u16> {
    match key {
        // Letters
        KeyCode::A => Some(0x00),
        KeyCode::B => Some(0x0B),
        KeyCode::C => Some(0x08),
        KeyCode::D => Some(0x02),
        KeyCode::E => Some(0x0E),
        KeyCode::F => Some(0x03),
        KeyCode::G => Some(0x05),
        KeyCode::H => Some(0x04),
        KeyCode::I => Some(0x22),
        KeyCode::J => Some(0x26),
        KeyCode::K => Some(0x28),
        KeyCode::L => Some(0x25),
        KeyCode::M => Some(0x2E),
        KeyCode::N => Some(0x2D),
        KeyCode::O => Some(0x1F),
        KeyCode::P => Some(0x23),
        KeyCode::Q => Some(0x0C),
        KeyCode::R => Some(0x0F),
        KeyCode::S => Some(0x01),
        KeyCode::T => Some(0x11),
        KeyCode::U => Some(0x20),
        KeyCode::V => Some(0x09),
        KeyCode::W => Some(0x0D),
        KeyCode::X => Some(0x07),
        KeyCode::Y => Some(0x10),
        KeyCode::Z => Some(0x06),

        // Top-row digits
        KeyCode::Key0 => Some(0x1D),
        KeyCode::Key1 => Some(0x12),
        KeyCode::Key2 => Some(0x13),
        KeyCode::Key3 => Some(0x14),
        KeyCode::Key4 => Some(0x15),
        KeyCode::Key5 => Some(0x17),
        KeyCode::Key6 => Some(0x16),
        KeyCode::Key7 => Some(0x1A),
        KeyCode::Key8 => Some(0x1C),
        KeyCode::Key9 => Some(0x19),

        // Function keys
        KeyCode::F1 => Some(0x7A),
        KeyCode::F2 => Some(0x78),
        KeyCode::F3 => Some(0x63),
        KeyCode::F4 => Some(0x76),
        KeyCode::F5 => Some(0x60),
        KeyCode::F6 => Some(0x61),
        KeyCode::F7 => Some(0x62),
        KeyCode::F8 => Some(0x64),
        KeyCode::F9 => Some(0x65),
        KeyCode::F10 => Some(0x6D),
        KeyCode::F11 => Some(0x67),
        KeyCode::F12 => Some(0x6F),
        KeyCode::F13 | KeyCode::PrintScreen => Some(0x69),
        KeyCode::F14 | KeyCode::ScrollLock => Some(0x6B),
        KeyCode::F15 | KeyCode::Pause => Some(0x71),
        KeyCode::F16 => Some(0x6A),
        KeyCode::F17 => Some(0x40),
        KeyCode::F18 => Some(0x4F),
        KeyCode::F19 => Some(0x50),
        KeyCode::F20 => Some(0x5A),
        KeyCode::F21 | KeyCode::F22 | KeyCode::F23 | KeyCode::F24 => None,

        // Modifiers: inject as left-hand variant.
        KeyCode::Ctrl => Some(0x3B),
        KeyCode::Shift => Some(0x38),
        KeyCode::Alt => Some(0x3A),
        KeyCode::Meta => Some(0x37),

        // Navigation and editing
        KeyCode::Space => Some(0x31),
        KeyCode::Enter => Some(0x24),
        KeyCode::Tab => Some(0x30),
        KeyCode::Escape => Some(0x35),
        KeyCode::Backspace => Some(0x33),
        KeyCode::Delete => Some(0x75),
        KeyCode::Insert => Some(0x72),
        KeyCode::Home => Some(0x73),
        KeyCode::End => Some(0x77),
        KeyCode::PageUp => Some(0x74),
        KeyCode::PageDown => Some(0x79),
        KeyCode::Up => Some(0x7E),
        KeyCode::Down => Some(0x7D),
        KeyCode::Left => Some(0x7B),
        KeyCode::Right => Some(0x7C),

        // Lock keys
        KeyCode::CapsLock => Some(0x39),
        KeyCode::NumLock => Some(0x47),

        // Numeric keypad
        KeyCode::Numpad0 => Some(0x52),
        KeyCode::Numpad1 => Some(0x53),
        KeyCode::Numpad2 => Some(0x54),
        KeyCode::Numpad3 => Some(0x55),
        KeyCode::Numpad4 => Some(0x56),
        KeyCode::Numpad5 => Some(0x57),
        KeyCode::Numpad6 => Some(0x58),
        KeyCode::Numpad7 => Some(0x59),
        KeyCode::Numpad8 => Some(0x5B),
        KeyCode::Numpad9 => Some(0x5C),
        KeyCode::NumpadAdd => Some(0x45),
        KeyCode::NumpadSub => Some(0x4E),
        KeyCode::NumpadMul => Some(0x43),
        KeyCode::NumpadDiv => Some(0x4B),
        KeyCode::NumpadEnter => Some(0x4C),

        // Punctuation / symbol keys
        KeyCode::Backtick => Some(0x32),
        KeyCode::Minus => Some(0x1B),
        KeyCode::Equal => Some(0x18),
        KeyCode::LeftBracket => Some(0x21),
        KeyCode::RightBracket => Some(0x1E),
        KeyCode::Backslash => Some(0x2A),
        KeyCode::Semicolon => Some(0x29),
        KeyCode::Apostrophe => Some(0x27),
        KeyCode::Comma => Some(0x2B),
        KeyCode::Period => Some(0x2F),
        KeyCode::Slash => Some(0x2C),
    }
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
        assert_eq!(vkcode_to_keycode(0x00), Some(KeyCode::A));
        assert_eq!(vkcode_to_keycode(0x0B), Some(KeyCode::B));
        assert_eq!(vkcode_to_keycode(0x06), Some(KeyCode::Z));
    }

    #[test]
    fn spot_check_digit_codes() {
        assert_eq!(vkcode_to_keycode(0x1D), Some(KeyCode::Key0));
        assert_eq!(vkcode_to_keycode(0x12), Some(KeyCode::Key1));
        assert_eq!(vkcode_to_keycode(0x19), Some(KeyCode::Key9));
    }

    #[test]
    fn spot_check_function_key_codes() {
        assert_eq!(vkcode_to_keycode(0x7A), Some(KeyCode::F1));
        assert_eq!(vkcode_to_keycode(0x6F), Some(KeyCode::F12));
        assert_eq!(vkcode_to_keycode(0x69), Some(KeyCode::F13));
        assert_eq!(vkcode_to_keycode(0x5A), Some(KeyCode::F20));
    }

    #[test]
    fn spot_check_navigation_keys() {
        assert_eq!(vkcode_to_keycode(0x7E), Some(KeyCode::Up));
        assert_eq!(vkcode_to_keycode(0x7D), Some(KeyCode::Down));
        assert_eq!(vkcode_to_keycode(0x73), Some(KeyCode::Home));
        assert_eq!(vkcode_to_keycode(0x77), Some(KeyCode::End));
        assert_eq!(vkcode_to_keycode(0x33), Some(KeyCode::Backspace));
        assert_eq!(vkcode_to_keycode(0x75), Some(KeyCode::Delete));
    }

    #[test]
    fn right_modifiers_map_to_canonical() {
        // Right-side variants must produce the same canonical KeyCode as left.
        assert_eq!(vkcode_to_keycode(0x3B), Some(KeyCode::Ctrl));
        assert_eq!(vkcode_to_keycode(0x3E), Some(KeyCode::Ctrl));
        assert_eq!(vkcode_to_keycode(0x38), Some(KeyCode::Shift));
        assert_eq!(vkcode_to_keycode(0x3C), Some(KeyCode::Shift));
        assert_eq!(vkcode_to_keycode(0x3A), Some(KeyCode::Alt));
        assert_eq!(vkcode_to_keycode(0x3D), Some(KeyCode::Alt));
        assert_eq!(vkcode_to_keycode(0x37), Some(KeyCode::Meta));
        assert_eq!(vkcode_to_keycode(0x36), Some(KeyCode::Meta));
    }

    #[test]
    fn unknown_vkcode_returns_none() {
        assert_eq!(vkcode_to_keycode(0xFF), None);
    }

    #[test]
    fn round_trip_primary_mappings() {
        // For each (KeyCode, expected vkcode) pair: both directions must agree.
        let cases: &[(KeyCode, u16)] = &[
            (KeyCode::A, 0x00),
            (KeyCode::Z, 0x06),
            (KeyCode::Key0, 0x1D),
            (KeyCode::Key9, 0x19),
            (KeyCode::F1, 0x7A),
            (KeyCode::F12, 0x6F),
            (KeyCode::F13, 0x69),
            (KeyCode::Ctrl, 0x3B),
            (KeyCode::Shift, 0x38),
            (KeyCode::Alt, 0x3A),
            (KeyCode::Meta, 0x37),
            (KeyCode::Enter, 0x24),
            (KeyCode::Space, 0x31),
            (KeyCode::Escape, 0x35),
            (KeyCode::Backspace, 0x33),
            (KeyCode::Delete, 0x75),
            (KeyCode::NumpadEnter, 0x4C),
            (KeyCode::Backtick, 0x32),
        ];
        for &(key, vk) in cases {
            assert_eq!(keycode_to_vkcode(key), Some(vk), "{key:?} -> vkcode");
            assert_eq!(
                vkcode_to_keycode(vk),
                Some(key),
                "vkcode {vk:#04x} -> keycode"
            );
        }
    }

    #[test]
    fn f21_f24_have_no_vkcode() {
        assert_eq!(keycode_to_vkcode(KeyCode::F21), None);
        assert_eq!(keycode_to_vkcode(KeyCode::F22), None);
        assert_eq!(keycode_to_vkcode(KeyCode::F23), None);
        assert_eq!(keycode_to_vkcode(KeyCode::F24), None);
    }

    #[test]
    fn printscreen_scrolllock_pause_map_to_f13_f14_f15() {
        assert_eq!(keycode_to_vkcode(KeyCode::PrintScreen), Some(0x69));
        assert_eq!(keycode_to_vkcode(KeyCode::ScrollLock), Some(0x6B));
        assert_eq!(keycode_to_vkcode(KeyCode::Pause), Some(0x71));
    }
}
