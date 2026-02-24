//! Hotkey lookup table: resolves held-key sets to exec actions at event time.

use std::collections::HashSet;

use crate::config::{HotkeyAction, HotkeyRule};
use crate::platform::{Action, KeyCode};

/// A compiled hotkey entry: all keys that must be held simultaneously, and the
/// action to fire when they are.
struct HotkeyEntry {
    keys: HashSet<KeyCode>,
    action: HotkeyAction,
    apps: Option<Vec<String>>,
}

impl HotkeyEntry {
    fn to_action(&self) -> Action {
        match &self.action {
            HotkeyAction::Exec(cmd) => Action::Exec {
                command: cmd.clone(),
            },
        }
    }
}

/// Compiled hotkey table. Per-app entries are stored before global entries so
/// that app-specific overrides win when window context is available (M11 readiness).
pub(super) struct HotkeyTable {
    entries: Vec<HotkeyEntry>,
}

impl HotkeyTable {
    pub(super) fn build(hotkeys: &[HotkeyRule]) -> Self {
        let mut entries: Vec<HotkeyEntry> = Vec::new();

        // Per-app rules first.
        for rule in hotkeys.iter().filter(|r| r.apps.is_some()) {
            entries.push(HotkeyEntry {
                keys: rule.keys.iter().copied().collect(),
                action: rule.action.clone(),
                apps: rule.apps.clone(),
            });
        }
        for rule in hotkeys.iter().filter(|r| r.apps.is_none()) {
            entries.push(HotkeyEntry {
                keys: rule.keys.iter().copied().collect(),
                action: rule.action.clone(),
                apps: rule.apps.clone(),
            });
        }

        Self { entries }
    }

    /// Find the first matching hotkey given the set of currently held keys.
    ///
    /// A hotkey matches when every key in its set is present in `held`.
    /// Per-app entries are checked first; the first matching global entry is
    /// the fallback. Returns `None` when no hotkey matches.
    /// Per-app entries are silently skipped when `app_id` is `None` (window
    /// context unavailable until M11).
    pub(super) fn lookup(&self, held: &HashSet<KeyCode>, app_id: Option<&str>) -> Option<Action> {
        let mut global_match: Option<&HotkeyEntry> = None;

        for entry in &self.entries {
            if !entry.keys.iter().all(|k| held.contains(k)) {
                continue;
            }

            match &entry.apps {
                Some(apps) => {
                    if let Some(id) = app_id {
                        if apps.iter().any(|a| a == id) {
                            return Some(entry.to_action());
                        }
                    }
                }
                None => {
                    if global_match.is_none() {
                        global_match = Some(entry);
                    }
                }
            }
        }

        global_match.map(|e| e.to_action())
    }
}
