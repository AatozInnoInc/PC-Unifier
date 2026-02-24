//! Remap lookup table: resolves `from` keys to `to` keys at event time.

use std::collections::HashMap;

use crate::config::RemapRule;
use crate::platform::KeyCode;

/// Compiled remap lookup table, keyed by the `from` key.
///
/// Within each entry, per-app rules are stored before global rules so that
/// app-specific overrides are evaluated first when window context is available
/// (M11 readiness). Config file order is preserved within each category.
pub(super) struct RemapTable {
    rules: HashMap<KeyCode, Vec<RemapRule>>,
}

impl RemapTable {
    pub(super) fn build(remaps: &[RemapRule]) -> Self {
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
    pub(super) fn lookup(&self, from: KeyCode, app_id: Option<&str>) -> Option<KeyCode> {
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
