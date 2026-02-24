//! Rule engine: match input events against compiled rules and produce actions.
//!
//! M8 implements remap rules. Hotkeys (M9), hotstrings (M10), and per-app
//! window filtering (M11) extend this module in later milestones. Lua script
//! handlers (M12/M13) will also route through here, supporting stateful and
//! multi-input scenarios (e.g. modifier + mouse button driving window moves).
//!
//! Rules are compiled into lookup tables at startup; `process` performs only
//! a hash lookup and never re-parses configuration.

use std::collections::HashMap;

use crate::config::{Config, RemapRule};
use crate::platform::{Action, InputEvent, KeyCode};

// ---------------------------------------------------------------------------
// Remap table (private)
// ---------------------------------------------------------------------------

/// Compiled remap lookup table, keyed by the `from` key.
///
/// Within each entry, per-app rules are stored before global rules so that
/// app-specific overrides are evaluated first when window context is available
/// (M11 readiness). Config file order is preserved within each category.
struct RemapTable {
    rules: HashMap<KeyCode, Vec<RemapRule>>,
}

impl RemapTable {
    fn build(remaps: &[RemapRule]) -> Self {
        let mut rules: HashMap<KeyCode, Vec<RemapRule>> = HashMap::new();

        // Per-app rules inserted first -- they win over globals when app matches.
        for rule in remaps.iter().filter(|r| r.apps.is_some()) {
            rules.entry(rule.from).or_default().push(rule.clone());
        }
        for rule in remaps.iter().filter(|r| r.apps.is_none()) {
            rules.entry(rule.from).or_default().push(rule.clone());
        }

        Self { rules }
    }

    /// Resolve `from` to a target key given the current app identifier.
    ///
    /// Per-app rules are evaluated first. The first matching global rule is
    /// the fallback. Returns `None` when no rule covers `from`.
    /// Per-app rules are silently skipped when `app_id` is `None` (window
    /// context unavailable until M11).
    fn lookup(&self, from: KeyCode, app_id: Option<&str>) -> Option<KeyCode> {
        let rules = self.rules.get(&from)?;
        let mut global_target: Option<KeyCode> = None;

        for rule in rules {
            match &rule.apps {
                Some(apps) => {
                    if let Some(id) = app_id {
                        if apps.iter().any(|a| a == id) {
                            return Some(rule.to);
                        }
                    }
                }
                None => {
                    if global_target.is_none() {
                        global_target = Some(rule.to);
                    }
                }
            }
        }

        global_target
    }
}

// ---------------------------------------------------------------------------
// Rule engine
// ---------------------------------------------------------------------------

/// Processes input events against compiled rules and produces actions.
///
/// Build once at startup with `RuleEngine::new`; the compiled tables are
/// immutable and require no synchronisation in the hot path.
pub struct RuleEngine {
    remaps: RemapTable,
}

impl RuleEngine {
    /// Build a `RuleEngine` from the parsed configuration.
    pub fn new(config: &Config) -> Self {
        Self {
            remaps: RemapTable::build(&config.remaps),
        }
    }

    /// Map an input event to an action.
    ///
    /// Evaluation order:
    ///   1. Remap rules -- per-app rules first (M11), then global.
    ///   2. Passthrough -- re-inject the original key unchanged.
    ///
    /// All platform backends suppress the original event at capture time, so
    /// passthrough is implemented as re-injection of the same key rather than
    /// `Action::Passthrough`. Per-app rules are silently skipped when
    /// `event.window.app_id` is `None` (window context unavailable until M11).
    pub fn process(&self, event: &InputEvent) -> Action {
        let app_id = event.window.app_id.as_deref();

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

    // --- Gate: A -> B ---

    #[test]
    fn global_remap_a_to_b() {
        let engine = engine_from_toml(
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

    // --- Passthrough ---

    #[test]
    fn unmapped_key_passes_through() {
        let engine = engine_from_toml(
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
        let engine = engine_from_toml("");
        assert_eq!(
            engine.process(&make_event(KeyCode::A)),
            Action::InjectKey {
                key: KeyCode::A,
                state: KeyState::Down
            }
        );
    }

    // --- Key state preserved ---

    #[test]
    fn remap_preserves_key_up_state() {
        let engine = engine_from_toml(
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

    // --- Multiple remaps ---

    #[test]
    fn multiple_remaps_each_independent() {
        let engine = engine_from_toml(
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

    // --- Per-app rules (window context stub until M11) ---

    #[test]
    fn per_app_remap_skipped_without_window_context() {
        let engine = engine_from_toml(
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
        let engine = engine_from_toml(
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
        let engine = engine_from_toml(
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
        let engine = engine_from_toml(
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
        let engine = engine_from_toml(
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

    // --- Higher-level smoke test: event_bus -> rule_engine pipeline ---

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
        let engine = RuleEngine::new(&config);

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
}
