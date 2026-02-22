# PC Unifier - Claude Code Context Brief

This document is the starting context for Claude Code. Read it fully before doing anything.

---

## What This Project Is

PC Unifier is a cross-platform input automation engine for Linux, macOS, and Windows.
It captures keyboard and mouse input at the OS level, matches events against user-defined
rules, and executes actions. It is not an AutoHotKey clone - the inspiration is AHK but
the audience is broader and the design is independent.

Repo: https://github.com/AatozInnoInc/PC-Unifier

---

## Key Decisions (Locked)

| Decision | Choice | Reason |
|---|---|---|
| Language | Rust | Memory safety for a system daemon, strong x-platform crate ecosystem |
| Scripting | Lua via mlua (LuaJIT) | Microsecond execution, small embed footprint, AI-friendly |
| Config format | TOML | Simple cases need zero scripting |
| Linux input | libei + xdg-desktop-portal | Modern Wayland path. No legacy X11-first design |
| Linux fallback | XWayland via XTest/XRecord | Detected automatically, logged as warning, zero user config |
| Distribution v1 | Static binary per platform, GitHub Releases | cargo build --release, no runtime deps |
| Distribution v2 | Homebrew, winget, AUR, Flatpak | After v1 is stable |
| GUI | v2 only | System tray and config editor deferred |
| License | MIT | Open source |

---

## Performance Constraint

Input capture to action execution must never exceed 33ms (30fps floor). This is a hard
gate at every milestone, not a post-release concern. The Lua hot path must not perform
I/O.

---

## Repo Structure

```
PC-Unifier/
├── CLAUDE.md
├── README.md
├── Cargo.toml                  # Workspace root
├── docs/
│   ├── architecture.md
│   ├── roadmap.md
│   └── config-schema.md
├── src/
...
└── .github/
```

---

## Config Schema (v1 Summary)

Four block types. All optional `apps` fields accept a string array.

```toml
[[remap]]
from = "Meta"
to   = "Ctrl"
apps = ["org.mozilla.firefox"]   # optional, multi-app

[[hotkey]]
keys    = ["Ctrl", "Alt", "T"]
action  = "exec"
command = "kitty"
apps    = ["org.gnome.Nautilus"] # optional, multi-app

[[hotstring]]
trigger     = ";;email"
replacement = "myemail@example.com"
apps        = ["org.mozilla.Thunderbird"] # optional, multi-app

[[script]]
path = "~/.config/pc-unifier/scripts/my_macros.lua"
```

Config file locations:
- Linux: `~/.config/pc-unifier/config.toml`
- macOS: `~/Library/Application Support/pc-unifier/config.toml`
- Windows: `%APPDATA%\pc-unifier\config.toml`

---

## Lua API Surface (v1)

```lua
pcunifier.remap("Meta", "Ctrl")
pcunifier.on_hotkey({"Ctrl", "Shift", "R"}, function(event) end)
pcunifier.hotstring(";;name", "Jane Doe")
pcunifier.on_key("Meta", function(event)
    return pcunifier.action.remap("Ctrl")   -- or passthrough() or suppress()
end)
pcunifier.focused_window()   -- returns { app_id, title }
pcunifier.exec("command")
```

---

## Platform Backend Summary

| OS | Capture | Emulation | Notes |
|---|---|---|---|
| Linux | xdg-desktop-portal Input Capture | libei | KDE 5.27+, GNOME 44+, Sway 1.8+, Hyprland 0.34+ |
| macOS | CGEventTap | CGEventPost | Requires Accessibility permission. Guide user on first run. |
| Windows | WH_KEYBOARD_LL / WH_MOUSE_LL | SendInput | No special permissions needed |

Linux startup flow:
1. Detect Wayland vs X11
2. Wayland: connect to xdg-desktop-portal Input Capture portal
3. Portal unavailable: detect XWayland, fall back with logged warning
4. No fallback available: exit with clear error listing supported compositors

---

## v1 Milestones (from docs/roadmap.md)

| # | Milestone | Gate |
|---|---|---|
| (DONE) M1 | Project scaffold + CI | `cargo build --release` on all 3 OS targets |
| (DONE) M2 | Platform trait definitions | Traits compile, type signature tests pass |
| M3 | Linux backend (Wayland) | Keypress captured and re-emitted, latency < 10ms |
| M4 | macOS backend | Same as M3 |
| M5 | Windows backend | Same as M3 |
| M6 | Event bus | 10k events, no drops, throughput logged |
| M7 | Config parser | Unit tests: valid, missing, wrong type, unknown keys |
| M8 | Rule engine: remaps | Integration test: A remapped to B |
| M9 | Rule engine: hotkeys | Integration test: combo triggers mock executor |
| M10 | Rule engine: hotstrings | Integration test: trigger expands to replacement |
| M11 | Window context | Per-app rule activates only on matching app |
| M12 | Lua runtime | Script loads, print() visible in daemon log |
| M13 | Lua API surface | Integration test covers every API function |
| M14 | Error messages + first-run UX | Manual walkthrough on each OS |
| M15 | Performance audit | All stages within 33ms budget, results in docs/benchmarks.md |

---

## Developer Notes

- The developer is a C++ Principal Engineer learning Rust for the first time.
  Explain Rust-specific syntax and concepts inline as you write code. Keep explanations
  brief and targeted - do not slow down implementation for tutorial tangents.
- Follow the instructions in AGENTS.md / system prompt strictly.
- Every milestone must fully pass its gate before the next begins.
- No sweeping changes. Keep diffs small and human-reviewable.
- No failing tests at any commit.