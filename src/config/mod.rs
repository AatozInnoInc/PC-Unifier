//! Config module: TOML config parser and validator.
//!
//! Two-pass design:
//!   1. `toml::from_str` deserializes raw TOML into private structs.
//!      `#[serde(deny_unknown_fields)]` rejects typos; the toml crate includes
//!      line and column in every error message.
//!   2. `validate` converts raw strings into typed values (`KeyCode`, `PathBuf`,
//!      `HotkeyAction`) and enforces cross-field constraints.
//!
//! Public entry points:
//!   - `parse_str(s)`           -- parse from a string (used in tests)
//!   - `load(path)`             -- read and validate from disk
//!   - `default_config_path()`  -- OS-conventional config file location

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::platform::KeyCode;

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// Errors that can occur when loading or validating a config file.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// I/O failure reading the config file.
    #[error("failed to read config file '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// TOML syntax error, unknown field, or missing required field.
    ///
    /// The toml crate includes line and column in the message, e.g.:
    /// `TOML parse error at line 3, column 5`
    #[error("config error: {0}")]
    Parse(#[from] toml::de::Error),

    /// A key name string is not recognized.
    #[error("unknown key name '{0}' -- see the config schema for valid key names")]
    UnknownKey(String),

    /// A hotkey `action` value is not recognized.
    #[error("unknown hotkey action '{0}' (valid actions: exec)")]
    UnknownAction(String),

    /// A `[[hotkey]]` with `action = "exec"` is missing the `command` field.
    #[error("hotkey with action 'exec' requires a 'command' field")]
    MissingCommand,

    /// An `apps` array is present but empty. Provide at least one identifier
    /// or remove the field for a global rule.
    #[error("apps field must contain at least one application identifier if present")]
    EmptyApps,
}

// ---------------------------------------------------------------------------
// Public typed output structs
// ---------------------------------------------------------------------------

/// A single `[[remap]]` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemapRule {
    pub from: KeyCode,
    pub to: KeyCode,
    /// `None` means the rule is global (applies to all applications).
    pub apps: Option<Vec<String>>,
}

/// The action performed by a `[[hotkey]]` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyAction {
    /// Spawn a shell command non-blocking.
    Exec(String),
}

/// A single `[[hotkey]]` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyRule {
    pub keys: Vec<KeyCode>,
    pub action: HotkeyAction,
    /// `None` means the rule is global.
    pub apps: Option<Vec<String>>,
}

/// A single `[[hotstring]]` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotstringRule {
    pub trigger: String,
    pub replacement: String,
    /// `None` means the rule is global.
    pub apps: Option<Vec<String>>,
}

/// A single `[[script]]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEntry {
    pub path: PathBuf,
}

/// The fully parsed and validated configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pub remaps: Vec<RemapRule>,
    pub hotkeys: Vec<HotkeyRule>,
    pub hotstrings: Vec<HotstringRule>,
    pub scripts: Vec<ScriptEntry>,
}

// ---------------------------------------------------------------------------
// Raw deserialization structs (private)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRemap {
    from: String,
    to: String,
    #[serde(default)]
    apps: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHotkey {
    keys: Vec<String>,
    action: String,
    command: Option<String>,
    #[serde(default)]
    apps: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHotstring {
    trigger: String,
    replacement: String,
    #[serde(default)]
    apps: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScript {
    path: String,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    remap: Vec<RawRemap>,
    #[serde(default)]
    hotkey: Vec<RawHotkey>,
    #[serde(default)]
    hotstring: Vec<RawHotstring>,
    #[serde(default)]
    script: Vec<RawScript>,
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Return the OS-conventional path for the config file.
///
/// | OS      | Path                                                        |
/// |---------|-------------------------------------------------------------|
/// | Linux   | `$XDG_CONFIG_HOME/pc-unifier/config.toml`                   |
/// | macOS   | `~/Library/Application Support/pc-unifier/config.toml`      |
/// | Windows | `%APPDATA%\pc-unifier\config.toml`                          |
pub fn default_config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Read and validate a config file from disk.
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_owned(),
        source,
    })?;
    parse_str(&text)
}

/// Parse and validate a config from a TOML string.
///
/// Exposed so tests can exercise the full validation pipeline without touching
/// the filesystem.
pub fn parse_str(s: &str) -> Result<Config, ConfigError> {
    let raw: RawConfig = toml::from_str(s)?;
    validate(raw)
}

// ---------------------------------------------------------------------------
// Validation (raw -> typed)
// ---------------------------------------------------------------------------

fn validate(raw: RawConfig) -> Result<Config, ConfigError> {
    let mut config = Config::default();

    for r in raw.remap {
        config.remaps.push(RemapRule {
            from: parse_key(&r.from)?,
            to: parse_key(&r.to)?,
            apps: validate_apps(r.apps)?,
        });
    }

    for h in raw.hotkey {
        let keys = h
            .keys
            .iter()
            .map(|k| parse_key(k))
            .collect::<Result<Vec<_>, _>>()?;
        let action = match h.action.as_str() {
            "exec" => HotkeyAction::Exec(h.command.ok_or(ConfigError::MissingCommand)?),
            other => return Err(ConfigError::UnknownAction(other.to_owned())),
        };
        config.hotkeys.push(HotkeyRule {
            keys,
            action,
            apps: validate_apps(h.apps)?,
        });
    }

    for s in raw.hotstring {
        config.hotstrings.push(HotstringRule {
            trigger: s.trigger,
            replacement: s.replacement,
            apps: validate_apps(s.apps)?,
        });
    }

    for s in raw.script {
        config.scripts.push(ScriptEntry {
            path: PathBuf::from(s.path),
        });
    }

    Ok(config)
}

/// Validate an optional `apps` array. If present it must be non-empty.
fn validate_apps(apps: Option<Vec<String>>) -> Result<Option<Vec<String>>, ConfigError> {
    match apps {
        Some(v) if v.is_empty() => Err(ConfigError::EmptyApps),
        other => Ok(other),
    }
}

// ---------------------------------------------------------------------------
// Key name resolution
// ---------------------------------------------------------------------------

/// Resolve a key name string to a `KeyCode`.
///
/// Matching is case-insensitive. Accepts canonical names, aliases from the
/// config schema (Control, Option, Super, Return, etc.), punctuation symbols,
/// and single-character letters/digits.
fn parse_key(s: &str) -> Result<KeyCode, ConfigError> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        // Modifiers and aliases
        "ctrl" | "control" => Ok(KeyCode::Ctrl),
        "shift" => Ok(KeyCode::Shift),
        "alt" | "option" => Ok(KeyCode::Alt),
        "meta" | "super" | "win" | "cmd" | "command" => Ok(KeyCode::Meta),

        // Letters
        "a" => Ok(KeyCode::A),
        "b" => Ok(KeyCode::B),
        "c" => Ok(KeyCode::C),
        "d" => Ok(KeyCode::D),
        "e" => Ok(KeyCode::E),
        "f" => Ok(KeyCode::F),
        "g" => Ok(KeyCode::G),
        "h" => Ok(KeyCode::H),
        "i" => Ok(KeyCode::I),
        "j" => Ok(KeyCode::J),
        "k" => Ok(KeyCode::K),
        "l" => Ok(KeyCode::L),
        "m" => Ok(KeyCode::M),
        "n" => Ok(KeyCode::N),
        "o" => Ok(KeyCode::O),
        "p" => Ok(KeyCode::P),
        "q" => Ok(KeyCode::Q),
        "r" => Ok(KeyCode::R),
        "s" => Ok(KeyCode::S),
        "t" => Ok(KeyCode::T),
        "u" => Ok(KeyCode::U),
        "v" => Ok(KeyCode::V),
        "w" => Ok(KeyCode::W),
        "x" => Ok(KeyCode::X),
        "y" => Ok(KeyCode::Y),
        "z" => Ok(KeyCode::Z),

        // Digits
        "0" => Ok(KeyCode::Key0),
        "1" => Ok(KeyCode::Key1),
        "2" => Ok(KeyCode::Key2),
        "3" => Ok(KeyCode::Key3),
        "4" => Ok(KeyCode::Key4),
        "5" => Ok(KeyCode::Key5),
        "6" => Ok(KeyCode::Key6),
        "7" => Ok(KeyCode::Key7),
        "8" => Ok(KeyCode::Key8),
        "9" => Ok(KeyCode::Key9),

        // Function keys
        "f1" => Ok(KeyCode::F1),
        "f2" => Ok(KeyCode::F2),
        "f3" => Ok(KeyCode::F3),
        "f4" => Ok(KeyCode::F4),
        "f5" => Ok(KeyCode::F5),
        "f6" => Ok(KeyCode::F6),
        "f7" => Ok(KeyCode::F7),
        "f8" => Ok(KeyCode::F8),
        "f9" => Ok(KeyCode::F9),
        "f10" => Ok(KeyCode::F10),
        "f11" => Ok(KeyCode::F11),
        "f12" => Ok(KeyCode::F12),
        "f13" => Ok(KeyCode::F13),
        "f14" => Ok(KeyCode::F14),
        "f15" => Ok(KeyCode::F15),
        "f16" => Ok(KeyCode::F16),
        "f17" => Ok(KeyCode::F17),
        "f18" => Ok(KeyCode::F18),
        "f19" => Ok(KeyCode::F19),
        "f20" => Ok(KeyCode::F20),
        "f21" => Ok(KeyCode::F21),
        "f22" => Ok(KeyCode::F22),
        "f23" => Ok(KeyCode::F23),
        "f24" => Ok(KeyCode::F24),

        // Navigation and editing
        "space" => Ok(KeyCode::Space),
        "enter" | "return" => Ok(KeyCode::Enter),
        "tab" => Ok(KeyCode::Tab),
        "escape" | "esc" => Ok(KeyCode::Escape),
        "backspace" => Ok(KeyCode::Backspace),
        "delete" | "del" => Ok(KeyCode::Delete),
        "insert" | "ins" => Ok(KeyCode::Insert),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "pageup" | "pgup" => Ok(KeyCode::PageUp),
        "pagedown" | "pgdn" | "pgdown" => Ok(KeyCode::PageDown),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),

        // Lock and system keys
        "capslock" => Ok(KeyCode::CapsLock),
        "numlock" => Ok(KeyCode::NumLock),
        "scrolllock" => Ok(KeyCode::ScrollLock),
        "printscreen" | "prtsc" | "prtscn" => Ok(KeyCode::PrintScreen),
        "pause" | "break" => Ok(KeyCode::Pause),

        // Numeric keypad
        "numpad0" => Ok(KeyCode::Numpad0),
        "numpad1" => Ok(KeyCode::Numpad1),
        "numpad2" => Ok(KeyCode::Numpad2),
        "numpad3" => Ok(KeyCode::Numpad3),
        "numpad4" => Ok(KeyCode::Numpad4),
        "numpad5" => Ok(KeyCode::Numpad5),
        "numpad6" => Ok(KeyCode::Numpad6),
        "numpad7" => Ok(KeyCode::Numpad7),
        "numpad8" => Ok(KeyCode::Numpad8),
        "numpad9" => Ok(KeyCode::Numpad9),
        "numpadadd" | "numpad+" => Ok(KeyCode::NumpadAdd),
        "numpadsub" | "numpad-" => Ok(KeyCode::NumpadSub),
        "numpadmul" | "numpad*" => Ok(KeyCode::NumpadMul),
        "numpaddiv" | "numpad/" => Ok(KeyCode::NumpadDiv),
        "numpadenter" => Ok(KeyCode::NumpadEnter),

        // Punctuation -- accept both the symbol and a spelled-out name
        "`" | "backtick" | "grave" => Ok(KeyCode::Backtick),
        "-" | "minus" | "hyphen" | "dash" => Ok(KeyCode::Minus),
        "=" | "equal" | "equals" => Ok(KeyCode::Equal),
        "[" | "leftbracket" | "lbracket" => Ok(KeyCode::LeftBracket),
        "]" | "rightbracket" | "rbracket" => Ok(KeyCode::RightBracket),
        "\\" | "backslash" => Ok(KeyCode::Backslash),
        ";" | "semicolon" => Ok(KeyCode::Semicolon),
        "'" | "apostrophe" | "quote" => Ok(KeyCode::Apostrophe),
        "," | "comma" => Ok(KeyCode::Comma),
        "." | "period" | "dot" => Ok(KeyCode::Period),
        "/" | "slash" => Ok(KeyCode::Slash),

        _ => Err(ConfigError::UnknownKey(s.to_owned())),
    }
}

// ---------------------------------------------------------------------------
// Config directory (platform-specific, no third-party deps)
// ---------------------------------------------------------------------------

fn config_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        // Respect XDG_CONFIG_HOME; fall back to ~/.config per XDG spec.
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("pc-unifier");
        }
        home_dir().join(".config").join("pc-unifier")
    }

    #[cfg(target_os = "macos")]
    {
        home_dir()
            .join("Library")
            .join("Application Support")
            .join("pc-unifier")
    }

    #[cfg(target_os = "windows")]
    {
        // APPDATA is always set on Windows; USERPROFILE is the fallback.
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("pc-unifier");
        }
        home_dir()
            .join("AppData")
            .join("Roaming")
            .join("pc-unifier")
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        PathBuf::from(".").join("pc-unifier")
    }
}

fn home_dir() -> PathBuf {
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:\\"))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Error assertion helpers ---
    //
    // Centralise variant matching so individual tests carry no hard-coded
    // failure messages and error-type changes require one fix point.

    fn assert_parse_err(result: Result<Config, ConfigError>) {
        match result.unwrap_err() {
            ConfigError::Parse(_) => {}
            other => panic!("expected ConfigError::Parse, got: {other}"),
        }
    }

    fn assert_missing_command(result: Result<Config, ConfigError>) {
        match result.unwrap_err() {
            ConfigError::MissingCommand => {}
            other => panic!("expected ConfigError::MissingCommand, got: {other}"),
        }
    }

    fn assert_unknown_key<'a>(result: Result<Config, ConfigError>, expected: &'a str) {
        match result.unwrap_err() {
            ConfigError::UnknownKey(k) if k == expected => {}
            other => panic!("expected ConfigError::UnknownKey({expected}), got: {other}"),
        }
    }

    fn assert_unknown_action<'a>(result: Result<Config, ConfigError>, expected: &'a str) {
        match result.unwrap_err() {
            ConfigError::UnknownAction(a) if a == expected => {}
            other => panic!("expected ConfigError::UnknownAction({expected}), got: {other}"),
        }
    }

    fn assert_empty_apps(result: Result<Config, ConfigError>) {
        match result.unwrap_err() {
            ConfigError::EmptyApps => {}
            other => panic!("expected ConfigError::EmptyApps, got: {other}"),
        }
    }

    // --- Valid configs ---

    #[test]
    fn valid_remap_minimal() {
        let cfg = parse_str(
            r#"
            [[remap]]
            from = "Meta"
            to   = "Ctrl"
        "#,
        )
        .unwrap();
        assert_eq!(cfg.remaps.len(), 1);
        assert_eq!(cfg.remaps[0].from, KeyCode::Meta);
        assert_eq!(cfg.remaps[0].to, KeyCode::Ctrl);
        assert!(cfg.remaps[0].apps.is_none());
    }

    #[test]
    fn valid_remap_with_apps() {
        let cfg = parse_str(
            r#"
            [[remap]]
            from = "Meta"
            to   = "Ctrl"
            apps = ["org.mozilla.firefox"]
        "#,
        )
        .unwrap();
        assert_eq!(
            cfg.remaps[0].apps.as_deref(),
            Some(&["org.mozilla.firefox".to_string()][..])
        );
    }

    #[test]
    fn valid_hotkey_exec() {
        let cfg = parse_str(
            r#"
            [[hotkey]]
            keys    = ["Ctrl", "Alt", "T"]
            action  = "exec"
            command = "kitty"
        "#,
        )
        .unwrap();
        assert_eq!(cfg.hotkeys.len(), 1);
        assert_eq!(
            cfg.hotkeys[0].keys,
            vec![KeyCode::Ctrl, KeyCode::Alt, KeyCode::T]
        );
        assert_eq!(cfg.hotkeys[0].action, HotkeyAction::Exec("kitty".into()));
        assert!(cfg.hotkeys[0].apps.is_none());
    }

    #[test]
    fn valid_hotstring() {
        let cfg = parse_str(
            r#"
            [[hotstring]]
            trigger     = ";;email"
            replacement = "me@example.com"
        "#,
        )
        .unwrap();
        assert_eq!(cfg.hotstrings.len(), 1);
        assert_eq!(cfg.hotstrings[0].trigger, ";;email");
        assert_eq!(cfg.hotstrings[0].replacement, "me@example.com");
    }

    #[test]
    fn valid_script() {
        let cfg = parse_str(
            r#"
            [[script]]
            path = "/home/user/.config/pc-unifier/macros.lua"
        "#,
        )
        .unwrap();
        assert_eq!(cfg.scripts.len(), 1);
        assert_eq!(
            cfg.scripts[0].path,
            PathBuf::from("/home/user/.config/pc-unifier/macros.lua")
        );
    }

    #[test]
    fn valid_empty_config() {
        let cfg = parse_str("").unwrap();
        assert!(cfg.remaps.is_empty());
        assert!(cfg.hotkeys.is_empty());
        assert!(cfg.hotstrings.is_empty());
        assert!(cfg.scripts.is_empty());
    }

    #[test]
    fn valid_full_config() {
        let cfg = parse_str(
            r#"
            [[remap]]
            from = "CapsLock"
            to   = "Escape"

            [[hotkey]]
            keys    = ["Meta", "L"]
            action  = "exec"
            command = "loginctl lock-session"

            [[hotstring]]
            trigger     = ";;sig"
            replacement = "Best regards"

            [[script]]
            path = "~/.config/pc-unifier/macros.lua"
        "#,
        )
        .unwrap();
        assert_eq!(cfg.remaps.len(), 1);
        assert_eq!(cfg.hotkeys.len(), 1);
        assert_eq!(cfg.hotstrings.len(), 1);
        assert_eq!(cfg.scripts.len(), 1);
    }

    // --- Missing required fields ---

    #[test]
    fn missing_remap_from() {
        assert_parse_err(parse_str(
            r#"
            [[remap]]
            to = "Ctrl"
        "#,
        ));
    }

    #[test]
    fn missing_remap_to() {
        assert_parse_err(parse_str(
            r#"
            [[remap]]
            from = "Meta"
        "#,
        ));
    }

    #[test]
    fn missing_hotkey_keys() {
        assert_parse_err(parse_str(
            r#"
            [[hotkey]]
            action  = "exec"
            command = "kitty"
        "#,
        ));
    }

    #[test]
    fn missing_hotkey_action() {
        assert_parse_err(parse_str(
            r#"
            [[hotkey]]
            keys = ["Ctrl", "T"]
        "#,
        ));
    }

    #[test]
    fn missing_hotkey_command_for_exec() {
        assert_missing_command(parse_str(
            r#"
            [[hotkey]]
            keys   = ["Ctrl", "T"]
            action = "exec"
        "#,
        ));
    }

    #[test]
    fn missing_hotstring_trigger() {
        assert_parse_err(parse_str(
            r#"
            [[hotstring]]
            replacement = "me@example.com"
        "#,
        ));
    }

    // --- Wrong types ---

    #[test]
    fn wrong_type_remap_from_integer() {
        assert_parse_err(parse_str(
            r#"
            [[remap]]
            from = 42
            to   = "Ctrl"
        "#,
        ));
    }

    #[test]
    fn wrong_type_hotkey_keys_not_array() {
        assert_parse_err(parse_str(
            r#"
            [[hotkey]]
            keys    = "Ctrl"
            action  = "exec"
            command = "kitty"
        "#,
        ));
    }

    // --- Unknown fields ---

    #[test]
    fn unknown_field_in_remap() {
        // "form" is a common typo for "from"
        assert_parse_err(parse_str(
            r#"
            [[remap]]
            form = "Meta"
            to   = "Ctrl"
        "#,
        ));
    }

    #[test]
    fn unknown_top_level_field() {
        assert_parse_err(parse_str(
            r#"
            [daemon]
            log_level = "debug"
        "#,
        ));
    }

    // --- Unknown key names ---

    #[test]
    fn unknown_key_name_in_remap() {
        assert_unknown_key(
            parse_str(
                r#"
                [[remap]]
                from = "NotAKey"
                to   = "Ctrl"
            "#,
            ),
            "NotAKey",
        );
    }

    #[test]
    fn unknown_hotkey_action() {
        assert_unknown_action(
            parse_str(
                r#"
                [[hotkey]]
                keys    = ["Ctrl", "T"]
                action  = "launch"
                command = "kitty"
            "#,
            ),
            "launch",
        );
    }

    // --- Empty apps array ---

    #[test]
    fn empty_apps_array() {
        assert_empty_apps(parse_str(
            r#"
            [[remap]]
            from = "Meta"
            to   = "Ctrl"
            apps = []
        "#,
        ));
    }

    // --- Key name aliases and case insensitivity ---

    #[test]
    fn key_name_aliases() {
        let cfg = parse_str(
            r#"
            [[hotkey]]
            keys   = ["Control", "Option", "Super", "Return"]
            action = "exec"
            command = "true"
        "#,
        )
        .unwrap();
        assert_eq!(
            cfg.hotkeys[0].keys,
            vec![KeyCode::Ctrl, KeyCode::Alt, KeyCode::Meta, KeyCode::Enter]
        );
    }

    #[test]
    fn key_names_case_insensitive() {
        let cfg = parse_str(
            r#"
            [[remap]]
            from = "CAPSLOCK"
            to   = "escape"
        "#,
        )
        .unwrap();
        assert_eq!(cfg.remaps[0].from, KeyCode::CapsLock);
        assert_eq!(cfg.remaps[0].to, KeyCode::Escape);
    }

    #[test]
    fn punctuation_symbol_keys() {
        let cfg = parse_str(
            r#"
            [[remap]]
            from = ";"
            to   = "apostrophe"
        "#,
        )
        .unwrap();
        assert_eq!(cfg.remaps[0].from, KeyCode::Semicolon);
        assert_eq!(cfg.remaps[0].to, KeyCode::Apostrophe);
    }
}
