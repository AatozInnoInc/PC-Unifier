//! Rule engine: match input events against compiled rules and produce actions.
//!
//! M8 implements remap rules. M9 adds hotkey support: modifier+key chords that
//! trigger exec actions. M10 adds hotstring support: typed character sequences
//! that expand to replacement text. Per-app window filtering (M11) and Lua
//! script handlers (M12/M13) will extend this module further.
//!
//! Rules are compiled into lookup tables at startup; `process` performs only
//! hash lookups and set membership tests, never re-parsing configuration.

mod hotkey;
mod hotstring;
mod keycodes;
mod remap;

use std::collections::HashSet;

use crate::config::Config;
use crate::platform::{Action, InputEvent, KeyCode, KeyState};
use hotkey::HotkeyTable;
use hotstring::{HotstringTable, InputBuffer};
use remap::RemapTable;

// ---------------------------------------------------------------------------
// Rule engine
// ---------------------------------------------------------------------------

/// Processes input events against compiled rules and produces actions.
///
/// Build once at startup with `RuleEngine::new`. `process` is `&mut self`
/// because it updates the transient held-key, suppression, and input-buffer
/// state that track chord and hotstring sequences across events.
pub struct RuleEngine {
    remaps: RemapTable,
    hotkeys: HotkeyTable,
    hotstrings: HotstringTable,
    /// Rolling window of recently typed printable characters for hotstring matching.
    input_buffer: InputBuffer,
    /// Keys currently held down. Updated on every KeyDown and KeyUp event.
    held_keys: HashSet<KeyCode>,
    /// Trigger keys whose KeyDown was consumed by a hotkey or hotstring match.
    /// The corresponding KeyUp is also suppressed to prevent ghost key-ups.
    suppressed_keys: HashSet<KeyCode>,
}

impl RuleEngine {
    /// Build a `RuleEngine` from the parsed configuration.
    pub fn new(config: &Config) -> Self {
        let hotstrings = HotstringTable::build(&config.hotstrings);
        let max_trigger_len = hotstrings.max_trigger_len;
        Self {
            remaps: RemapTable::build(&config.remaps),
            hotkeys: HotkeyTable::build(&config.hotkeys),
            hotstrings,
            input_buffer: InputBuffer::new(max_trigger_len),
            held_keys: HashSet::new(),
            suppressed_keys: HashSet::new(),
        }
    }

    /// Map an input event to an action.
    ///
    /// On KeyDown, evaluation order:
    ///   1. Hotkey rules -- fires when all chord keys are held; per-app rules
    ///      first (M11 readiness), then global. The trigger key is suppressed.
    ///   2. Remap rules -- per-app first (M11), then global.
    ///   3. Passthrough -- re-inject the original key unchanged.
    ///
    /// On KeyUp:
    ///   1. Suppress if the corresponding KeyDown was consumed by a hotkey.
    ///   2. Remap / passthrough as for KeyDown.
    ///
    /// All platform backends suppress the original event at capture time, so
    /// passthrough is implemented as re-injection rather than `Action::Passthrough`.
    /// Per-app rules are silently skipped when `event.window.app_id` is `None`
    /// (window context unavailable until M11).
    pub fn process(&mut self, event: &InputEvent) -> Action {
        let app_id = event.window.app_id.as_deref();

        match event.state {
            KeyState::Down => {
                self.held_keys.insert(event.key);

                // 1. Hotkeys take priority: a chord match short-circuits everything else.
                //    The hotkey trigger key's Down is consumed and its Up will be suppressed.
                //    The buffer is NOT updated -- hotkey keys should not appear in typed text.
                if let Some(action) = self.hotkeys.lookup(&self.held_keys, app_id) {
                    log::debug!("rule_engine: hotkey fired on {:?}: {:?}", event.key, action);
                    self.suppressed_keys.insert(event.key);
                    return action;
                }

                // 2. Update the rolling input buffer and check for hotstring triggers.
                //    Shift state is derived from held_keys (already updated above).
                //    map() is used to avoid holding an immutable borrow on self.hotstrings
                //    while mutating self.input_buffer and self.suppressed_keys below.
                let shift_held = self.held_keys.contains(&KeyCode::Shift);
                if self.input_buffer.push(event.key, shift_held) {
                    let hotstring_match = self
                        .hotstrings
                        .check(self.input_buffer.as_str(), app_id)
                        .map(|(n, s)| (n, s.to_string()));

                    if let Some((backspaces, replacement)) = hotstring_match {
                        log::debug!(
                            "rule_engine: hotstring fired on {:?}: {} backspace(s) + \"{}\"",
                            event.key,
                            backspaces,
                            replacement
                        );
                        self.input_buffer.clear();
                        self.suppressed_keys.insert(event.key);
                        return Action::Hotstring {
                            backspaces,
                            replacement,
                        };
                    }
                }

                // 3. Remap rules.
                if let Some(target) = self.remaps.lookup(event.key, app_id) {
                    log::debug!(
                        "rule_engine: remap {:?} -> {:?} ({:?})",
                        event.key,
                        target,
                        event.state
                    );
                    return Action::InjectKey {
                        key: target,
                        state: event.state,
                    };
                }

                // 4. Passthrough.
                Action::InjectKey {
                    key: event.key,
                    state: event.state,
                }
            }

            KeyState::Up => {
                self.held_keys.remove(&event.key);

                // Suppress the KeyUp for any key whose KeyDown was consumed by a
                // hotkey or hotstring match.
                if self.suppressed_keys.remove(&event.key) {
                    log::debug!(
                        "rule_engine: suppressing KeyUp for consumed key {:?}",
                        event.key
                    );
                    return Action::Suppress;
                }

                if let Some(target) = self.remaps.lookup(event.key, app_id) {
                    log::debug!(
                        "rule_engine: remap {:?} -> {:?} ({:?})",
                        event.key,
                        target,
                        event.state
                    );
                    return Action::InjectKey {
                        key: target,
                        state: event.state,
                    };
                }

                Action::InjectKey {
                    key: event.key,
                    state: event.state,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{KeyState, Modifiers, WindowContext};

    fn make_event(key: KeyCode) -> InputEvent {
        InputEvent {
            key,
            state: KeyState::Down,
            modifiers: Modifiers::default(),
            window: WindowContext::default(),
        }
    }

    fn make_event_with_state(key: KeyCode, state: KeyState) -> InputEvent {
        InputEvent {
            key,
            state,
            modifiers: Modifiers::default(),
            window: WindowContext::default(),
        }
    }

    fn make_event_with_app(key: KeyCode, app_id: &str) -> InputEvent {
        InputEvent {
            key,
            state: KeyState::Down,
            modifiers: Modifiers::default(),
            window: WindowContext {
                app_id: Some(app_id.to_string()),
                title: None,
            },
        }
    }

    fn engine_from_toml(toml: &str) -> RuleEngine {
        let config = crate::config::parse_str(toml).unwrap();
        RuleEngine::new(&config)
    }

    // --- Remap tests (M8) ---

    #[test]
    fn global_remap_a_to_b() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "A"
            to   = "B"
        "#,
        );
        assert_eq!(
            engine.process(&make_event(KeyCode::A)),
            Action::InjectKey {
                key: KeyCode::B,
                state: KeyState::Down
            }
        );
    }

    #[test]
    fn unmapped_key_passes_through() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "A"
            to   = "B"
        "#,
        );
        assert_eq!(
            engine.process(&make_event(KeyCode::C)),
            Action::InjectKey {
                key: KeyCode::C,
                state: KeyState::Down
            }
        );
    }

    #[test]
    fn empty_config_key_passes_through() {
        let mut engine = engine_from_toml("");
        assert_eq!(
            engine.process(&make_event(KeyCode::A)),
            Action::InjectKey {
                key: KeyCode::A,
                state: KeyState::Down
            }
        );
    }

    #[test]
    fn remap_preserves_key_up_state() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "A"
            to   = "B"
        "#,
        );
        let mut event = make_event(KeyCode::A);
        event.state = KeyState::Up;
        assert_eq!(
            engine.process(&event),
            Action::InjectKey {
                key: KeyCode::B,
                state: KeyState::Up
            }
        );
    }

    #[test]
    fn multiple_remaps_each_independent() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "A"
            to   = "B"

            [[remap]]
            from = "Ctrl"
            to   = "Meta"
        "#,
        );
        assert_eq!(
            engine.process(&make_event(KeyCode::A)),
            Action::InjectKey {
                key: KeyCode::B,
                state: KeyState::Down
            }
        );
        assert_eq!(
            engine.process(&make_event(KeyCode::Ctrl)),
            Action::InjectKey {
                key: KeyCode::Meta,
                state: KeyState::Down
            }
        );
    }

    #[test]
    fn per_app_remap_skipped_without_window_context() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "A"
            to   = "B"
            apps = ["org.mozilla.firefox"]
        "#,
        );
        // app_id is None until M11 -- per-app rule must not activate.
        assert_eq!(
            engine.process(&make_event(KeyCode::A)),
            Action::InjectKey {
                key: KeyCode::A,
                state: KeyState::Down
            }
        );
    }

    #[test]
    fn per_app_remap_activates_when_app_matches() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "A"
            to   = "B"
            apps = ["org.mozilla.firefox"]
        "#,
        );
        assert_eq!(
            engine.process(&make_event_with_app(KeyCode::A, "org.mozilla.firefox")),
            Action::InjectKey {
                key: KeyCode::B,
                state: KeyState::Down
            }
        );
    }

    #[test]
    fn per_app_remap_skipped_for_different_app() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "A"
            to   = "B"
            apps = ["org.mozilla.firefox"]
        "#,
        );
        assert_eq!(
            engine.process(&make_event_with_app(KeyCode::A, "org.gnome.Nautilus")),
            Action::InjectKey {
                key: KeyCode::A,
                state: KeyState::Down
            }
        );
    }

    #[test]
    fn per_app_rule_overrides_global_when_app_matches() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "Meta"
            to   = "Ctrl"
            apps = ["org.mozilla.firefox"]

            [[remap]]
            from = "Meta"
            to   = "Alt"
        "#,
        );
        assert_eq!(
            engine.process(&make_event_with_app(KeyCode::Meta, "org.mozilla.firefox")),
            Action::InjectKey {
                key: KeyCode::Ctrl,
                state: KeyState::Down
            }
        );
        assert_eq!(
            engine.process(&make_event(KeyCode::Meta)),
            Action::InjectKey {
                key: KeyCode::Alt,
                state: KeyState::Down
            }
        );
    }

    /// Two global rules with the same `from` key: the first in config order wins.
    /// NOTE: In future releases (maybe v2) we should explicitly validate against this behavior!
    #[test]
    fn two_global_rules_same_from_key_first_wins() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "A"
            to   = "B"

            [[remap]]
            from = "A"
            to   = "C"
        "#,
        );
        assert_eq!(
            engine.process(&make_event(KeyCode::A)),
            Action::InjectKey {
                key: KeyCode::B,
                state: KeyState::Down
            }
        );
    }

    // --- Hotkey tests (M9) ---

    /// Gate test: Ctrl+Alt+T fires an exec action when all three keys are held.
    #[test]
    fn hotkey_ctrl_alt_t_fires_exec() {
        let mut engine = engine_from_toml(
            r#"
            [[hotkey]]
            keys    = ["Ctrl", "Alt", "T"]
            action  = "exec"
            command = "kitty"
        "#,
        );
        engine.process(&make_event(KeyCode::Ctrl));
        engine.process(&make_event(KeyCode::Alt));
        let action = engine.process(&make_event(KeyCode::T));
        assert_eq!(
            action,
            Action::Exec {
                command: "kitty".into()
            }
        );
    }

    /// The trigger key's Up is suppressed after a hotkey fires.
    #[test]
    fn hotkey_trigger_key_up_is_suppressed() {
        let mut engine = engine_from_toml(
            r#"
            [[hotkey]]
            keys    = ["Ctrl", "Alt", "T"]
            action  = "exec"
            command = "kitty"
        "#,
        );
        engine.process(&make_event(KeyCode::Ctrl));
        engine.process(&make_event(KeyCode::Alt));
        engine.process(&make_event(KeyCode::T)); // fires hotkey, suppresses T Down
        let up_action = engine.process(&make_event_with_state(KeyCode::T, KeyState::Up));
        assert_eq!(up_action, Action::Suppress);
    }

    /// An incomplete chord (missing one key) does not fire the hotkey.
    #[test]
    fn hotkey_incomplete_chord_does_not_fire() {
        let mut engine = engine_from_toml(
            r#"
            [[hotkey]]
            keys    = ["Ctrl", "Alt", "T"]
            action  = "exec"
            command = "kitty"
        "#,
        );
        // Only Ctrl held, not Alt.
        engine.process(&make_event(KeyCode::Ctrl));
        let action = engine.process(&make_event(KeyCode::T));
        assert_eq!(
            action,
            Action::InjectKey {
                key: KeyCode::T,
                state: KeyState::Down
            }
        );
    }

    /// Hotkeys do not affect unrelated key presses.
    #[test]
    fn hotkey_unrelated_key_passes_through() {
        let mut engine = engine_from_toml(
            r#"
            [[hotkey]]
            keys    = ["Ctrl", "Alt", "T"]
            action  = "exec"
            command = "kitty"
        "#,
        );
        let action = engine.process(&make_event(KeyCode::A));
        assert_eq!(
            action,
            Action::InjectKey {
                key: KeyCode::A,
                state: KeyState::Down
            }
        );
    }

    /// Hotkeys take priority over remaps for the same trigger key.
    #[test]
    fn hotkey_fires_before_remap() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "T"
            to   = "B"

            [[hotkey]]
            keys    = ["Ctrl", "T"]
            action  = "exec"
            command = "kitty"
        "#,
        );
        engine.process(&make_event(KeyCode::Ctrl));
        let action = engine.process(&make_event(KeyCode::T));
        assert_eq!(
            action,
            Action::Exec {
                command: "kitty".into()
            }
        );
    }

    /// Per-app hotkey is not active when no window context is present (until M11).
    #[test]
    fn per_app_hotkey_skipped_without_window_context() {
        let mut engine = engine_from_toml(
            r#"
            [[hotkey]]
            keys    = ["Ctrl", "T"]
            action  = "exec"
            command = "kitty"
            apps    = ["org.gnome.Terminal"]
        "#,
        );
        engine.process(&make_event(KeyCode::Ctrl));
        let action = engine.process(&make_event(KeyCode::T));
        assert_eq!(
            action,
            Action::InjectKey {
                key: KeyCode::T,
                state: KeyState::Down
            }
        );
    }

    /// Per-app hotkey fires when window context matches.
    #[test]
    fn per_app_hotkey_activates_when_app_matches() {
        let mut engine = engine_from_toml(
            r#"
            [[hotkey]]
            keys    = ["Ctrl", "T"]
            action  = "exec"
            command = "kitty"
            apps    = ["org.gnome.Terminal"]
        "#,
        );
        engine.process(&make_event_with_app(KeyCode::Ctrl, "org.gnome.Terminal"));
        let action = engine.process(&make_event_with_app(KeyCode::T, "org.gnome.Terminal"));
        assert_eq!(
            action,
            Action::Exec {
                command: "kitty".into()
            }
        );
    }

    // --- Hotstring tests (M10) ---

    /// Gate test: typing ;;email fires Action::Hotstring with the right payload.
    #[test]
    fn hotstring_semicolons_email_expands() {
        let mut engine = engine_from_toml(
            r#"
            [[hotstring]]
            trigger     = ";;email"
            replacement = "myemail@example.com"
        "#,
        );

        // First six keys pass through; final key fires the hotstring.
        for key in [
            KeyCode::Semicolon,
            KeyCode::Semicolon,
            KeyCode::E,
            KeyCode::M,
            KeyCode::A,
            KeyCode::I,
        ] {
            engine.process(&make_event(key));
        }

        let action = engine.process(&make_event(KeyCode::L));
        assert_eq!(
            action,
            Action::Hotstring {
                backspaces: 6,
                replacement: "myemail@example.com".into(),
            }
        );
    }

    /// The final trigger key's KeyUp is suppressed after a hotstring fires.
    #[test]
    fn hotstring_trigger_key_up_is_suppressed() {
        let mut engine = engine_from_toml(
            r#"
            [[hotstring]]
            trigger     = ";;email"
            replacement = "myemail@example.com"
        "#,
        );
        for key in [
            KeyCode::Semicolon,
            KeyCode::Semicolon,
            KeyCode::E,
            KeyCode::M,
            KeyCode::A,
            KeyCode::I,
            KeyCode::L,
        ] {
            engine.process(&make_event(key));
        }
        let up_action = engine.process(&make_event_with_state(KeyCode::L, KeyState::Up));
        assert_eq!(up_action, Action::Suppress);
    }

    /// A non-printable key (Escape) clears the buffer; the sequence must be
    /// restarted from the beginning.
    #[test]
    fn hotstring_buffer_clears_on_escape_then_retyped_matches() {
        let mut engine = engine_from_toml(
            r#"
            [[hotstring]]
            trigger     = ";;email"
            replacement = "myemail@example.com"
        "#,
        );
        // Type partial trigger, interrupt with Escape.
        engine.process(&make_event(KeyCode::Semicolon));
        engine.process(&make_event(KeyCode::Semicolon));
        // Escape clears the buffer; the full trigger must be restarted.
        engine.process(&make_event(KeyCode::Escape));
        for key in [
            KeyCode::Semicolon,
            KeyCode::Semicolon,
            KeyCode::E,
            KeyCode::M,
            KeyCode::A,
            KeyCode::I,
        ] {
            engine.process(&make_event(key));
        }
        let action = engine.process(&make_event(KeyCode::L));
        assert_eq!(
            action,
            Action::Hotstring {
                backspaces: 6,
                replacement: "myemail@example.com".into(),
            }
        );
    }

    /// Pressing and releasing Shift does not clear the buffer; sequence still matches.
    #[test]
    fn hotstring_modifier_does_not_clear_buffer() {
        let mut engine = engine_from_toml(
            r#"
            [[hotstring]]
            trigger     = ";;email"
            replacement = "myemail@example.com"
        "#,
        );
        // Type ;;, press and release Shift (should not clear), then continue unshifted.
        engine.process(&make_event(KeyCode::Semicolon));
        engine.process(&make_event(KeyCode::Semicolon));
        engine.process(&make_event(KeyCode::Shift));
        engine.process(&make_event_with_state(KeyCode::Shift, KeyState::Up));
        for key in [KeyCode::E, KeyCode::M, KeyCode::A, KeyCode::I] {
            engine.process(&make_event(key));
        }
        let action = engine.process(&make_event(KeyCode::L));
        assert_eq!(
            action,
            Action::Hotstring {
                backspaces: 6,
                replacement: "myemail@example.com".into(),
            }
        );
    }

    /// Shift-modified characters are recorded correctly; `!email` (Shift+1 then
    /// unshifted letters) must match a trigger of `"!email"`.
    #[test]
    fn hotstring_shifted_prefix_expands() {
        let mut engine = engine_from_toml(
            r#"
            [[hotstring]]
            trigger     = "!email"
            replacement = "myemail@example.com"
        "#,
        );

        // '!' = Shift+Key1: press Shift first, then Key1 while Shift is held.
        engine.process(&make_event(KeyCode::Shift)); // held_keys gains Shift
        engine.process(&make_event(KeyCode::Key1)); // buffer records '!'
        engine.process(&make_event_with_state(KeyCode::Shift, KeyState::Up)); // Shift released

        // Unshifted letters.
        for key in [KeyCode::E, KeyCode::M, KeyCode::A, KeyCode::I] {
            engine.process(&make_event(key));
        }

        let action = engine.process(&make_event(KeyCode::L));
        assert_eq!(
            action,
            Action::Hotstring {
                backspaces: 5,
                replacement: "myemail@example.com".into(),
            }
        );
    }

    /// Space clears the buffer; a partial trigger followed by Space does not expand.
    #[test]
    fn hotstring_space_clears_buffer_no_match() {
        let mut engine = engine_from_toml(
            r#"
            [[hotstring]]
            trigger     = ";;email"
            replacement = "myemail@example.com"
        "#,
        );
        engine.process(&make_event(KeyCode::Semicolon));
        engine.process(&make_event(KeyCode::Semicolon));
        // Space clears the buffer; the remaining chars alone should not match.
        engine.process(&make_event(KeyCode::Space));
        for key in [KeyCode::E, KeyCode::M, KeyCode::A, KeyCode::I, KeyCode::L] {
            let action = engine.process(&make_event(key));
            assert_ne!(
                action,
                Action::Hotstring {
                    backspaces: 6,
                    replacement: "myemail@example.com".into(),
                }
            );
        }
    }

    /// Hotstrings take priority over remaps for the same key sequence.
    #[test]
    fn hotstring_fires_before_remap() {
        let mut engine = engine_from_toml(
            r#"
            [[remap]]
            from = "L"
            to   = "K"

            [[hotstring]]
            trigger     = ";;email"
            replacement = "myemail@example.com"
        "#,
        );
        for key in [
            KeyCode::Semicolon,
            KeyCode::Semicolon,
            KeyCode::E,
            KeyCode::M,
            KeyCode::A,
            KeyCode::I,
        ] {
            engine.process(&make_event(key));
        }
        // L has a remap rule but the hotstring trigger takes priority.
        let action = engine.process(&make_event(KeyCode::L));
        assert_eq!(
            action,
            Action::Hotstring {
                backspaces: 6,
                replacement: "myemail@example.com".into(),
            }
        );
    }

    // --- Higher-level smoke tests: event_bus -> rule_engine pipeline ---

    #[test]
    fn smoke_bus_to_rule_engine_remap() {
        // Verifies the integration path from EventPublisher through RuleEngine
        // without any platform I/O.
        let config = crate::config::parse_str(
            r#"
            [[remap]]
            from = "A"
            to   = "B"
        "#,
        )
        .unwrap();
        let mut engine = RuleEngine::new(&config);

        let (publisher, mut subscriber) = crate::event_bus::new(8);
        publisher.send(InputEvent {
            key: KeyCode::A,
            state: KeyState::Down,
            modifiers: Modifiers::default(),
            window: WindowContext::default(),
        });
        drop(publisher);

        let event = subscriber.next().unwrap();
        assert_eq!(
            engine.process(&event),
            Action::InjectKey {
                key: KeyCode::B,
                state: KeyState::Down
            }
        );
    }

    /// Gate smoke test: event_bus -> rule_engine pipeline fires a hotkey exec action.
    #[test]
    fn smoke_bus_hotkey_fires_exec() {
        let config = crate::config::parse_str(
            r#"
            [[hotkey]]
            keys    = ["Ctrl", "Alt", "T"]
            action  = "exec"
            command = "kitty"
        "#,
        )
        .unwrap();
        let mut engine = RuleEngine::new(&config);

        let (publisher, mut subscriber) = crate::event_bus::new(8);
        publisher.send(InputEvent {
            key: KeyCode::Ctrl,
            state: KeyState::Down,
            modifiers: Modifiers::default(),
            window: WindowContext::default(),
        });
        publisher.send(InputEvent {
            key: KeyCode::Alt,
            state: KeyState::Down,
            modifiers: Modifiers::default(),
            window: WindowContext::default(),
        });
        publisher.send(InputEvent {
            key: KeyCode::T,
            state: KeyState::Down,
            modifiers: Modifiers::default(),
            window: WindowContext::default(),
        });
        drop(publisher);

        engine.process(&subscriber.next().unwrap()); // Ctrl Down
        engine.process(&subscriber.next().unwrap()); // Alt Down
        let action = engine.process(&subscriber.next().unwrap()); // T Down -> hotkey fires
        assert_eq!(
            action,
            Action::Exec {
                command: "kitty".into()
            }
        );
    }
}
