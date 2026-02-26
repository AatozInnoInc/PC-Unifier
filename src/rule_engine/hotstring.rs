//! Hotstring lookup table and rolling input buffer.
//!
//! `HotstringTable` is compiled once at startup from `[[hotstring]]` config
//! entries. `InputBuffer` maintains a rolling window of recently typed
//! characters; after each printable key event the rule engine calls
//! `HotstringTable::check` to test for a trigger match.
//!
//! Key classification (printable vs. modifier vs. clear) is handled by
//! `rule_engine::keycodes`, which mirrors the role that `platform/*/keycodes.rs`
//! plays for the platform backends.

use super::keycodes::{classify_key, KeyClass};
use crate::config::HotstringRule;
use crate::platform::KeyCode;

// ---------------------------------------------------------------------------
// Input buffer
// ---------------------------------------------------------------------------

/// Rolling typed-character buffer used for hotstring trigger matching.
///
/// Grows up to `max_len` characters (the length of the longest configured
/// trigger). Characters are appended for printable key events (unshifted
/// value), ignored for modifier keys, and the buffer is cleared for Space,
/// Backspace, Tab, Enter, Escape, and all other non-printable keys.
///
/// The buffer never exceeds `max_len`, discarding the oldest characters to
/// maintain only the most recent suffix of the typed sequence.
pub(super) struct InputBuffer {
    buf: String,
    max_len: usize,
}

impl InputBuffer {
    /// Create a new buffer sized for the longest configured trigger.
    ///
    /// When `max_len` is 0 (no hotstrings configured), the buffer is a no-op:
    /// printable chars are appended and immediately trimmed away.
    pub(super) fn new(max_len: usize) -> Self {
        Self {
            buf: String::with_capacity(max_len),
            max_len,
        }
    }

    /// Push a key event into the buffer.
    ///
    /// `shift_held` controls whether the shifted character variant is recorded
    /// (e.g. `Key1` with `shift_held=true` appends `'!'` rather than `'1'`).
    ///
    /// Returns `true` if a printable character was appended -- the caller
    /// should then call `HotstringTable::check` to test for a match.
    /// Returns `false` for modifier keys (no change) and clearing keys (buffer
    /// was reset).
    pub(super) fn push(&mut self, key: KeyCode, shift_held: bool) -> bool {
        match classify_key(key, shift_held) {
            KeyClass::Printable(c) => {
                self.buf.push(c);
                if self.buf.len() > self.max_len {
                    let excess = self.buf.len() - self.max_len;
                    // SAFETY: classify_key only produces ASCII characters (single-byte
                    // codepoints), so byte-indexed drain never splits a codepoint.
                    // Non-ASCII triggers would require char-count-based trimming.
                    self.buf.drain(..excess);
                }
                log::debug!(
                    "hotstring buffer: {:?} (shift={}) -> append '{}', state=\"{}\" (max {})",
                    key,
                    shift_held,
                    c,
                    self.buf,
                    self.max_len
                );
                true
            }
            KeyClass::Modifier => {
                log::debug!(
                    "hotstring buffer: {:?} is a modifier, buffer unchanged (\"{}\")",
                    key,
                    self.buf
                );
                false
            }
            KeyClass::Clear => {
                if !self.buf.is_empty() {
                    log::debug!(
                        "hotstring buffer: {:?} clears buffer (was \"{}\")",
                        key,
                        self.buf
                    );
                    // Design: any non-printable key (including Backspace) resets the full
                    // buffer. This models line-start semantics: a correction means restart
                    // from scratch. Inline edit (remove last char only) is not modeled.
                    self.buf.clear();
                }
                false
            }
        }
    }

    pub(super) fn as_str(&self) -> &str {
        &self.buf
    }

    pub(super) fn clear(&mut self) {
        log::debug!("hotstring buffer: explicit clear (was \"{}\")", self.buf);
        self.buf.clear();
    }
}

// ---------------------------------------------------------------------------
// Hotstring table
// ---------------------------------------------------------------------------

/// A compiled hotstring entry.
struct HotstringEntry {
    trigger: String,
    replacement: String,
    apps: Option<Vec<String>>,
}

/// Compiled hotstring lookup table.
///
/// Per-app entries are stored before global entries so that app-specific
/// overrides win when window context is available (M11 readiness).
pub(super) struct HotstringTable {
    entries: Vec<HotstringEntry>,
    /// Length of the longest trigger; used to size the `InputBuffer`.
    pub(super) max_trigger_len: usize,
}

impl HotstringTable {
    pub(super) fn build(hotstrings: &[HotstringRule]) -> Self {
        let mut entries: Vec<HotstringEntry> = Vec::new();
        let mut max_trigger_len: usize = 0;

        // Per-app rules first (M11 readiness).
        for rule in hotstrings
            .iter()
            .filter(|r| r.apps.is_some() && !r.trigger.is_empty())
        {
            max_trigger_len = max_trigger_len.max(rule.trigger.len());
            entries.push(HotstringEntry {
                trigger: rule.trigger.clone(),
                replacement: rule.replacement.clone(),
                apps: rule.apps.clone(),
            });
        }
        for rule in hotstrings
            .iter()
            .filter(|r| r.apps.is_none() && !r.trigger.is_empty())
        {
            max_trigger_len = max_trigger_len.max(rule.trigger.len());
            entries.push(HotstringEntry {
                trigger: rule.trigger.clone(),
                replacement: rule.replacement.clone(),
                apps: None,
            });
        }

        log::debug!(
            "hotstring: compiled {} entries, max trigger len {}",
            entries.len(),
            max_trigger_len
        );

        Self {
            entries,
            max_trigger_len,
        }
    }

    /// Check whether the buffer ends with a configured trigger.
    ///
    /// Per-app entries are evaluated first; the first matching global entry is
    /// the fallback. Per-app entries are silently skipped when `app_id` is
    /// `None` (window context unavailable until M11).
    ///
    /// Returns `(backspaces, replacement)` on a match, where
    /// `backspaces == trigger.len() - 1`. The final trigger character is
    /// suppressed at the rule engine level and must not be counted among the
    /// backspaces.
    ///
    /// Logs each trigger tested and the final outcome to aid debugging of
    /// false positives and missed matches.
    pub(super) fn check(&self, buf: &str, app_id: Option<&str>) -> Option<(usize, &str)> {
        if self.entries.is_empty() {
            return None;
        }

        log::debug!(
            "hotstring: check buf=\"{}\" ({} chars) against {} trigger(s), app={:?}",
            buf,
            buf.len(),
            self.entries.len(),
            app_id
        );

        let mut global_match: Option<&HotstringEntry> = None;

        for entry in &self.entries {
            let suffix_matches = buf.ends_with(entry.trigger.as_str());
            log::debug!(
                "hotstring:   trigger=\"{}\" suffix_match={} apps={:?}",
                entry.trigger,
                suffix_matches,
                entry.apps
            );

            if !suffix_matches {
                continue;
            }

            match &entry.apps {
                Some(apps) => {
                    if let Some(id) = app_id {
                        if apps.iter().any(|a| a == id) {
                            log::debug!(
                                "hotstring: per-app match: trigger=\"{}\" app=\"{}\" \
                                 -> {} backspace(s) + \"{}\"",
                                entry.trigger,
                                id,
                                entry.trigger.chars().count() - 1,
                                entry.replacement
                            );
                            return Some((entry.trigger.chars().count() - 1, &entry.replacement));
                        }
                    }
                    log::debug!(
                        "hotstring:   trigger=\"{}\" app mismatch (have {:?}, want {:?})",
                        entry.trigger,
                        app_id,
                        apps
                    );
                }
                None => {
                    if global_match.is_none() {
                        log::debug!(
                            "hotstring:   global candidate: trigger=\"{}\"",
                            entry.trigger
                        );
                        global_match = Some(entry);
                    }
                }
            }
        }

        match global_match {
            Some(e) => {
                log::debug!(
                    "hotstring: global match: trigger=\"{}\" -> {} backspace(s) + \"{}\"",
                    e.trigger,
                    e.trigger.chars().count() - 1,
                    e.replacement
                );
                Some((e.trigger.chars().count() - 1, e.replacement.as_str()))
            }
            None => {
                log::debug!("hotstring: no match for buf=\"{}\"", buf);
                None
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
    use crate::config::HotstringRule;

    fn rule(trigger: &str, replacement: &str) -> HotstringRule {
        HotstringRule {
            trigger: trigger.into(),
            replacement: replacement.into(),
            apps: None,
        }
    }

    fn rule_with_app(trigger: &str, replacement: &str, app: &str) -> HotstringRule {
        HotstringRule {
            trigger: trigger.into(),
            replacement: replacement.into(),
            apps: Some(vec![app.into()]),
        }
    }

    // --- InputBuffer ---

    #[test]
    fn buffer_appends_printable_keys() {
        let mut buf = InputBuffer::new(10);
        assert!(buf.push(KeyCode::Semicolon, false));
        assert!(buf.push(KeyCode::E, false));
        assert_eq!(buf.as_str(), ";e");
    }

    #[test]
    fn buffer_records_shifted_characters() {
        let mut buf = InputBuffer::new(10);
        // Shift+1 -> '!', Shift+A -> 'A'
        assert!(buf.push(KeyCode::Key1, true));
        assert!(buf.push(KeyCode::A, true));
        assert_eq!(buf.as_str(), "!A");
    }

    #[test]
    fn buffer_ignores_modifier_keys() {
        let mut buf = InputBuffer::new(10);
        buf.push(KeyCode::Semicolon, false);
        assert!(!buf.push(KeyCode::Shift, false));
        assert_eq!(buf.as_str(), ";"); // unchanged
    }

    #[test]
    fn buffer_clears_on_space() {
        let mut buf = InputBuffer::new(10);
        buf.push(KeyCode::Semicolon, false);
        buf.push(KeyCode::Semicolon, false);
        assert!(!buf.push(KeyCode::Space, false));
        assert_eq!(buf.as_str(), "");
    }

    #[test]
    fn buffer_clears_on_backspace() {
        let mut buf = InputBuffer::new(10);
        buf.push(KeyCode::A, false);
        assert!(!buf.push(KeyCode::Backspace, false));
        assert_eq!(buf.as_str(), "");
    }

    #[test]
    fn buffer_clears_on_tab() {
        let mut buf = InputBuffer::new(10);
        buf.push(KeyCode::A, false);
        assert!(!buf.push(KeyCode::Tab, false));
        assert_eq!(buf.as_str(), "");
    }

    #[test]
    fn buffer_clears_on_enter() {
        let mut buf = InputBuffer::new(10);
        buf.push(KeyCode::A, false);
        assert!(!buf.push(KeyCode::Enter, false));
        assert_eq!(buf.as_str(), "");
    }

    #[test]
    fn buffer_clears_on_numpad_enter() {
        let mut buf = InputBuffer::new(10);
        buf.push(KeyCode::A, false);
        assert!(!buf.push(KeyCode::NumpadEnter, false));
        assert_eq!(buf.as_str(), "");
    }

    #[test]
    fn buffer_clears_on_escape() {
        let mut buf = InputBuffer::new(10);
        buf.push(KeyCode::A, false);
        assert!(!buf.push(KeyCode::Escape, false));
        assert_eq!(buf.as_str(), "");
    }

    #[test]
    fn buffer_trims_to_max_len() {
        let mut buf = InputBuffer::new(3);
        buf.push(KeyCode::A, false);
        buf.push(KeyCode::B, false);
        buf.push(KeyCode::C, false);
        buf.push(KeyCode::D, false);
        assert_eq!(buf.as_str(), "bcd");
    }

    #[test]
    fn buffer_zero_max_len_never_accumulates() {
        let mut buf = InputBuffer::new(0);
        buf.push(KeyCode::A, false);
        assert_eq!(buf.as_str(), "");
    }

    // --- HotstringTable ---

    #[test]
    fn table_matches_trigger_at_end_of_buffer() {
        let table = HotstringTable::build(&[rule(";;email", "me@example.com")]);
        assert_eq!(table.check(";;email", None), Some((6, "me@example.com")));
    }

    #[test]
    fn table_matches_trigger_within_longer_buffer() {
        let table = HotstringTable::build(&[rule(";;email", "me@example.com")]);
        // Trigger appears at the end of a longer accumulated buffer.
        assert_eq!(
            table.check("hello;;email", None),
            Some((6, "me@example.com"))
        );
    }

    #[test]
    fn table_no_match_on_partial_trigger() {
        let table = HotstringTable::build(&[rule(";;email", "me@example.com")]);
        assert!(table.check(";;emai", None).is_none());
    }

    #[test]
    fn table_empty_returns_none() {
        let table = HotstringTable::build(&[]);
        assert!(table.check(";;email", None).is_none());
    }

    #[test]
    fn max_trigger_len_reflects_longest_trigger() {
        let table = HotstringTable::build(&[rule("ab", "x"), rule("abcde", "y")]);
        assert_eq!(table.max_trigger_len, 5);
    }

    #[test]
    fn per_app_rule_matches_when_app_matches() {
        let table = HotstringTable::build(&[rule_with_app(";;em", "me@example.com", "org.app")]);
        assert_eq!(
            table.check(";;em", Some("org.app")),
            Some((3, "me@example.com"))
        );
    }

    #[test]
    fn per_app_rule_skipped_when_app_differs() {
        let table = HotstringTable::build(&[rule_with_app(";;em", "me@example.com", "org.app")]);
        assert!(table.check(";;em", Some("org.other")).is_none());
    }

    #[test]
    fn per_app_rule_skipped_without_app_context() {
        let table = HotstringTable::build(&[rule_with_app(";;em", "me@example.com", "org.app")]);
        assert!(table.check(";;em", None).is_none());
    }

    #[test]
    fn global_rule_is_fallback_when_app_does_not_match() {
        let table = HotstringTable::build(&[
            rule_with_app(";;em", "app_specific@example.com", "org.app"),
            rule(";;em", "global@example.com"),
        ]);
        assert_eq!(
            table.check(";;em", Some("org.other")),
            Some((3, "global@example.com"))
        );
    }
}
