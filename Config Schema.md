# PC Unifier - Config Schema Reference

## Overview

PC Unifier is configured via a TOML file. For most users, this file is all that is
needed. Lua scripting is available for cases that require conditional logic.

**Config file locations:**

| OS | Path |
|---|---|
| Linux | `~/.config/pc-unifier/config.toml` |
| macOS | `~/Library/Application Support/pc-unifier/config.toml` |
| Windows | `%APPDATA%\pc-unifier\config.toml` |

PC Unifier creates the file with defaults on first run if it does not exist.

---

## Key Names

Key names are case-insensitive strings. The following names are recognized:

**Modifier keys:**

| Name | Aliases |
|---|---|
| `Ctrl` | `Control` |
| `Shift` | |
| `Alt` | `Option` (macOS alias) |
| `Meta` | `Super`, `Win`, `Cmd`, `Command` |

**Standard keys:** `A`-`Z`, `0`-`9`, `F1`-`F24`

**Special keys:**

`Space`, `Enter`, `Return`, `Tab`, `Escape`, `Backspace`, `Delete`,
`Insert`, `Home`, `End`, `PageUp`, `PageDown`,
`Up`, `Down`, `Left`, `Right`,
`CapsLock`, `NumLock`, `ScrollLock`, `PrintScreen`, `Pause`,
`Numpad0`-`Numpad9`, `NumpadAdd`, `NumpadSub`, `NumpadMul`, `NumpadDiv`, `NumpadEnter`,
`` ` ``, `-`, `=`, `[`, `]`, `\`, `;`, `'`, `,`, `.`, `/`

---

## App Identifiers

The `apps` field targets one or more specific applications. The identifier format differs by OS:

| OS | Format | Example |
|---|---|---|
| Linux | WM_CLASS or application ID | `org.mozilla.firefox`, `firefox` |
| macOS | Bundle identifier | `org.mozilla.firefox` |
| Windows | Executable name | `firefox.exe` |

Run `pcunifier --list-windows` to print the identifiers of all open windows.

---

## `[[remap]]`

Remap one key to another. The source key is suppressed and the target key is injected.

```toml
[[remap]]
from = "Meta"           # required  - key to intercept
to   = "Ctrl"           # required  - key to inject instead
apps = ["firefox.exe"]    # optional  - limit to one or more applications
```

**Fields:**

| Field | Type | Required | Description |
|---|---|---|---|
| `from` | string | Yes | Key name to intercept |
| `to` | string | Yes | Key name to inject |
| `apps` | string array | No | Application identifiers. Omit for global remap. |

**Example - Mac-style close for Firefox on Linux:**
```toml
[[remap]]
from = "Meta"
to   = "Ctrl"
apps = ["org.mozilla.firefox"]
```

**Example - Swap Caps Lock and Escape (popular with Vim users):**
```toml
[[remap]]
from = "CapsLock"
to   = "Escape"

[[remap]]
from = "Escape"
to   = "CapsLock"
```

---

## `[[hotkey]]`

Trigger an action when a key combination is pressed.

```toml
[[hotkey]]
keys    = ["Ctrl", "Alt", "T"]   # required  - key combination
action  = "exec"                  # required  - action type
command = "kitty"                 # required for exec
apps    = ["org.gnome.Nautilus"]   # optional  - limit to one or more applications
```

**Fields:**

| Field | Type | Required | Description |
|---|---|---|---|
| `keys` | string array | Yes | Key combination. Order does not matter for modifiers. |
| `action` | string | Yes | Action to perform. See action types below. |
| `command` | string | When `action = "exec"` | Shell command to run. |
| `apps` | string array | No | Application identifiers. Omit for global hotkey. |

**Action types:**

| Action | Description |
|---|---|
| `exec` | Run a shell command. Requires `command` field. Non-blocking. |

**Example - Open terminal:**
```toml
[[hotkey]]
keys    = ["Ctrl", "Alt", "T"]
action  = "exec"
command = "kitty"
```

**Example - Take a screenshot (Linux):**
```toml
[[hotkey]]
keys    = ["Meta", "Shift", "S"]
action  = "exec"
command = "grimblast copy area"
```

---

## `[[hotstring]]`

Expand a typed sequence into a replacement string. The trigger is suppressed and the
replacement is injected.

```toml
[[hotstring]]
trigger     = ";;email"                  # required  - sequence to detect
replacement = "myemail@example.com"      # required  - text to inject
apps        = ["org.mozilla.Thunderbird"]  # optional  - limit to one or more applications
```

**Fields:**

| Field | Type | Required | Description |
|---|---|---|---|
| `trigger` | string | Yes | Character sequence to detect. |
| `replacement` | string | Yes | Text to inject after trigger is detected. |
| `apps` | string array | No | Application identifiers. Omit for global hotstring. |

**Notes:**
- The trigger is matched as typed, character by character.
- On match, the trigger characters are deleted and the replacement is typed.
- There is no required delimiter. The trigger fires as soon as the sequence is complete.
- Use a unique prefix (e.g. `;;`) to avoid accidental triggers.

**Example - Personal snippets:**
```toml
[[hotstring]]
trigger     = ";;name"
replacement = "Jane Doe"

[[hotstring]]
trigger     = ";;addr"
replacement = "123 Main Street, Springfield"

[[hotstring]]
trigger     = ";;date"
replacement = "2025-01-15"
```

---

## `[[script]]`

Load a Lua script. Scripts can register any rule that the TOML config supports, plus
conditional logic based on the focused window or other state.

```toml
[[script]]
path = "~/.config/pc-unifier/scripts/my_macros.lua"   # required
```

**Fields:**

| Field | Type | Required | Description |
|---|---|---|---|
| `path` | string | Yes | Absolute or `~`-prefixed path to a `.lua` file. |

Multiple `[[script]]` blocks are allowed. Scripts are loaded in order.

See the [Lua API documentation](lua-api.md) for available functions.

---

## Full Example

```toml
# ~/.config/pc-unifier/config.toml

# Mac-style Meta key for users switching from macOS to Linux
[[remap]]
from = "Meta"
to   = "Ctrl"

# Swap Caps Lock to Escape for Vim
[[remap]]
from = "CapsLock"
to   = "Escape"

# Open terminal
[[hotkey]]
keys    = ["Ctrl", "Alt", "T"]
action  = "exec"
command = "kitty"

# Lock screen
[[hotkey]]
keys    = ["Meta", "L"]
action  = "exec"
command = "loginctl lock-session"

# Text snippets
[[hotstring]]
trigger     = ";;email"
replacement = "myemail@example.com"

[[hotstring]]
trigger     = ";;sig"
replacement = "Best regards,\nJane Doe"

# Advanced logic via Lua
[[script]]
path = "~/.config/pc-unifier/scripts/app_specific.lua"
```

---

## Validation and Errors

PC Unifier validates the config at startup. Errors are printed to stderr with the
line number and a description. The daemon does not start with an invalid config.

Common errors:

| Error | Cause |
|---|---|
| `unknown field 'form'` | Typo in field name. Did you mean `from`? |
| `missing field 'to'` | Required field omitted |
| `unknown key name 'CTRL'` | Key name not recognized. Check capitalization. |
| `apps field present but empty` | Provide at least one value or remove the field |

Run `pcunifier --validate` to check your config without starting the daemon.
