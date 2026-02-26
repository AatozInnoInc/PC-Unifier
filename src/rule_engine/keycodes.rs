//! KeyCode classification for the hotstring input buffer.
//!
//! Maps `KeyCode` variants to printable characters, modifier status (ignored
//! by the buffer), or a clear signal (resets the buffer). This is the
//! rule-engine equivalent of the platform-level `keycodes.rs` files.
//!
//! Shift awareness: `classify_key` takes the current Shift held state and
//! returns the shifted character when appropriate (e.g. Shift+1 -> '!',
//! Shift+A -> 'A'). Mappings follow a standard US QWERTY layout.
//!
//! Buffer-clear keys include: Space, Backspace, Tab, Enter (both variants),
//! and Escape. All other non-printable, non-modifier keys also clear the
//! buffer (function keys, navigation, numpad, etc.).
//!
//! CapsLock: CapsLock state is not tracked. Letters are classified based on
//! the Shift key alone. A trigger typed with CapsLock active (e.g. ";;EMAIL")
//! will not match a lowercase-configured trigger (e.g. ";;email").

use crate::platform::KeyCode;

// ---------------------------------------------------------------------------
// Key classification
// ---------------------------------------------------------------------------

/// How a key event should affect the rolling hotstring input buffer.
pub(super) enum KeyClass {
    /// Append this unshifted character to the buffer.
    Printable(char),
    /// Ignore; leave the buffer unchanged (modifier keys).
    Modifier,
    /// Clear the buffer.
    ///
    /// Triggered by Space, Backspace, Tab, Enter (both variants), Escape, and
    /// all other non-printable, non-modifier keys (function keys, navigation,
    /// numpad, lock keys, etc.).
    Clear,
}

/// Classify a `KeyCode` for hotstring buffer purposes.
///
/// Returns `Modifier` for Ctrl, Shift, Alt, and Meta (buffer unchanged).
/// Returns `Clear` for Space, Backspace, Tab, Enter, Escape, and all other
/// non-printable keys (buffer reset).
/// Returns `Printable(c)` for all other keys, where `c` is the shifted or
/// unshifted character depending on `shift_held` (US QWERTY layout).
pub(super) fn classify_key(key: KeyCode, shift_held: bool) -> KeyClass {
    match key {
        // Modifiers: ignored by the buffer
        KeyCode::Ctrl | KeyCode::Shift | KeyCode::Alt | KeyCode::Meta => KeyClass::Modifier,

        // Letters: lowercase normally, uppercase when Shift is held
        KeyCode::A => KeyClass::Printable(if shift_held { 'A' } else { 'a' }),
        KeyCode::B => KeyClass::Printable(if shift_held { 'B' } else { 'b' }),
        KeyCode::C => KeyClass::Printable(if shift_held { 'C' } else { 'c' }),
        KeyCode::D => KeyClass::Printable(if shift_held { 'D' } else { 'd' }),
        KeyCode::E => KeyClass::Printable(if shift_held { 'E' } else { 'e' }),
        KeyCode::F => KeyClass::Printable(if shift_held { 'F' } else { 'f' }),
        KeyCode::G => KeyClass::Printable(if shift_held { 'G' } else { 'g' }),
        KeyCode::H => KeyClass::Printable(if shift_held { 'H' } else { 'h' }),
        KeyCode::I => KeyClass::Printable(if shift_held { 'I' } else { 'i' }),
        KeyCode::J => KeyClass::Printable(if shift_held { 'J' } else { 'j' }),
        KeyCode::K => KeyClass::Printable(if shift_held { 'K' } else { 'k' }),
        KeyCode::L => KeyClass::Printable(if shift_held { 'L' } else { 'l' }),
        KeyCode::M => KeyClass::Printable(if shift_held { 'M' } else { 'm' }),
        KeyCode::N => KeyClass::Printable(if shift_held { 'N' } else { 'n' }),
        KeyCode::O => KeyClass::Printable(if shift_held { 'O' } else { 'o' }),
        KeyCode::P => KeyClass::Printable(if shift_held { 'P' } else { 'p' }),
        KeyCode::Q => KeyClass::Printable(if shift_held { 'Q' } else { 'q' }),
        KeyCode::R => KeyClass::Printable(if shift_held { 'R' } else { 'r' }),
        KeyCode::S => KeyClass::Printable(if shift_held { 'S' } else { 's' }),
        KeyCode::T => KeyClass::Printable(if shift_held { 'T' } else { 't' }),
        KeyCode::U => KeyClass::Printable(if shift_held { 'U' } else { 'u' }),
        KeyCode::V => KeyClass::Printable(if shift_held { 'V' } else { 'v' }),
        KeyCode::W => KeyClass::Printable(if shift_held { 'W' } else { 'w' }),
        KeyCode::X => KeyClass::Printable(if shift_held { 'X' } else { 'x' }),
        KeyCode::Y => KeyClass::Printable(if shift_held { 'Y' } else { 'y' }),
        KeyCode::Z => KeyClass::Printable(if shift_held { 'Z' } else { 'z' }),

        // Top-row digits: digit normally, shifted symbol when Shift is held (US QWERTY)
        KeyCode::Key0 => KeyClass::Printable(if shift_held { ')' } else { '0' }),
        KeyCode::Key1 => KeyClass::Printable(if shift_held { '!' } else { '1' }),
        KeyCode::Key2 => KeyClass::Printable(if shift_held { '@' } else { '2' }),
        KeyCode::Key3 => KeyClass::Printable(if shift_held { '#' } else { '3' }),
        KeyCode::Key4 => KeyClass::Printable(if shift_held { '$' } else { '4' }),
        KeyCode::Key5 => KeyClass::Printable(if shift_held { '%' } else { '5' }),
        KeyCode::Key6 => KeyClass::Printable(if shift_held { '^' } else { '6' }),
        KeyCode::Key7 => KeyClass::Printable(if shift_held { '&' } else { '7' }),
        KeyCode::Key8 => KeyClass::Printable(if shift_held { '*' } else { '8' }),
        KeyCode::Key9 => KeyClass::Printable(if shift_held { '(' } else { '9' }),

        // Punctuation: unshifted and shifted variants (US QWERTY)
        KeyCode::Backtick => KeyClass::Printable(if shift_held { '~' } else { '`' }),
        KeyCode::Minus => KeyClass::Printable(if shift_held { '_' } else { '-' }),
        KeyCode::Equal => KeyClass::Printable(if shift_held { '+' } else { '=' }),
        KeyCode::LeftBracket => KeyClass::Printable(if shift_held { '{' } else { '[' }),
        KeyCode::RightBracket => KeyClass::Printable(if shift_held { '}' } else { ']' }),
        KeyCode::Backslash => KeyClass::Printable(if shift_held { '|' } else { '\\' }),
        KeyCode::Semicolon => KeyClass::Printable(if shift_held { ':' } else { ';' }),
        KeyCode::Apostrophe => KeyClass::Printable(if shift_held { '"' } else { '\'' }),
        KeyCode::Comma => KeyClass::Printable(if shift_held { '<' } else { ',' }),
        KeyCode::Period => KeyClass::Printable(if shift_held { '>' } else { '.' }),
        KeyCode::Slash => KeyClass::Printable(if shift_held { '?' } else { '/' }),

        // Space, Backspace, Tab, Enter (and NumpadEnter), Escape: clear the buffer.
        // All remaining keys (function, navigation, numpad, lock, etc.) also clear.
        _ => KeyClass::Clear,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn is_printable(key: KeyCode, shift_held: bool) -> bool {
        matches!(classify_key(key, shift_held), KeyClass::Printable(_))
    }

    fn is_modifier(key: KeyCode) -> bool {
        matches!(classify_key(key, false), KeyClass::Modifier)
    }

    fn is_clear(key: KeyCode) -> bool {
        matches!(classify_key(key, false), KeyClass::Clear)
    }

    fn char_for(key: KeyCode, shift_held: bool) -> char {
        match classify_key(key, shift_held) {
            KeyClass::Printable(c) => c,
            _ => panic!("{key:?} is not printable"),
        }
    }

    // --- Modifiers ---

    #[test]
    fn modifiers_are_ignored() {
        assert!(is_modifier(KeyCode::Ctrl));
        assert!(is_modifier(KeyCode::Shift));
        assert!(is_modifier(KeyCode::Alt));
        assert!(is_modifier(KeyCode::Meta));
    }

    // --- Letters ---

    #[test]
    fn letters_are_printable_lowercase_unshifted() {
        assert_eq!(char_for(KeyCode::A, false), 'a');
        assert_eq!(char_for(KeyCode::Z, false), 'z');
        assert_eq!(char_for(KeyCode::M, false), 'm');
    }

    #[test]
    fn letters_are_uppercase_when_shift_held() {
        assert_eq!(char_for(KeyCode::A, true), 'A');
        assert_eq!(char_for(KeyCode::Z, true), 'Z');
        assert_eq!(char_for(KeyCode::M, true), 'M');
    }

    // --- Digits and shifted symbols ---

    #[test]
    fn digits_are_printable_unshifted() {
        assert_eq!(char_for(KeyCode::Key0, false), '0');
        assert_eq!(char_for(KeyCode::Key9, false), '9');
    }

    #[test]
    fn digit_keys_produce_shifted_symbols() {
        assert_eq!(char_for(KeyCode::Key1, true), '!');
        assert_eq!(char_for(KeyCode::Key2, true), '@');
        assert_eq!(char_for(KeyCode::Key3, true), '#');
        assert_eq!(char_for(KeyCode::Key4, true), '$');
        assert_eq!(char_for(KeyCode::Key5, true), '%');
        assert_eq!(char_for(KeyCode::Key6, true), '^');
        assert_eq!(char_for(KeyCode::Key7, true), '&');
        assert_eq!(char_for(KeyCode::Key8, true), '*');
        assert_eq!(char_for(KeyCode::Key9, true), '(');
        assert_eq!(char_for(KeyCode::Key0, true), ')');
    }

    // --- Punctuation ---

    #[test]
    fn semicolon_maps_to_semicolon_unshifted_colon_shifted() {
        assert_eq!(char_for(KeyCode::Semicolon, false), ';');
        assert_eq!(char_for(KeyCode::Semicolon, true), ':');
    }

    #[test]
    fn punctuation_keys_are_printable() {
        for key in [
            KeyCode::Backtick,
            KeyCode::Minus,
            KeyCode::Equal,
            KeyCode::LeftBracket,
            KeyCode::RightBracket,
            KeyCode::Backslash,
            KeyCode::Apostrophe,
            KeyCode::Comma,
            KeyCode::Period,
            KeyCode::Slash,
        ] {
            assert!(is_printable(key, false), "{key:?} should be printable");
            assert!(
                is_printable(key, true),
                "{key:?} should be printable when shifted"
            );
        }
    }

    #[test]
    fn punctuation_shifted_variants() {
        assert_eq!(char_for(KeyCode::Backtick, true), '~');
        assert_eq!(char_for(KeyCode::Minus, true), '_');
        assert_eq!(char_for(KeyCode::Equal, true), '+');
        assert_eq!(char_for(KeyCode::LeftBracket, true), '{');
        assert_eq!(char_for(KeyCode::RightBracket, true), '}');
        assert_eq!(char_for(KeyCode::Backslash, true), '|');
        assert_eq!(char_for(KeyCode::Apostrophe, true), '"');
        assert_eq!(char_for(KeyCode::Comma, true), '<');
        assert_eq!(char_for(KeyCode::Period, true), '>');
        assert_eq!(char_for(KeyCode::Slash, true), '?');
    }

    // --- Buffer-clearing keys ---

    #[test]
    fn space_clears_buffer() {
        assert!(is_clear(KeyCode::Space));
    }

    #[test]
    fn backspace_clears_buffer() {
        assert!(is_clear(KeyCode::Backspace));
    }

    #[test]
    fn tab_clears_buffer() {
        assert!(is_clear(KeyCode::Tab));
    }

    #[test]
    fn enter_clears_buffer() {
        assert!(is_clear(KeyCode::Enter));
    }

    #[test]
    fn numpad_enter_clears_buffer() {
        assert!(is_clear(KeyCode::NumpadEnter));
    }

    #[test]
    fn escape_clears_buffer() {
        assert!(is_clear(KeyCode::Escape));
    }

    #[test]
    fn navigation_keys_clear_buffer() {
        for key in [
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::PageUp,
            KeyCode::PageDown,
            KeyCode::Delete,
            KeyCode::Insert,
        ] {
            assert!(is_clear(key), "{key:?} should clear the buffer");
        }
    }

    #[test]
    fn function_keys_clear_buffer() {
        assert!(is_clear(KeyCode::F1));
        assert!(is_clear(KeyCode::F12));
    }
}
