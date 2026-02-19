# PC Unifier

A cross-platform input automation engine for Linux, macOS, and Windows.

Remap keys, create hotkeys, expand text snippets, and automate repetitive input -
configured in a simple TOML file or extended with Lua scripts.

---

## Why PC Unifier

Switching between operating systems means relearning muscle memory. Key positions
differ, shortcuts conflict, and productivity tools rarely work across all three
platforms. PC Unifier solves this at the input layer.

**Common use cases:**

- Remap Meta to Ctrl on Linux for users coming from macOS
- Create consistent shortcuts across all your machines
- Expand short text triggers into full snippets
- Trigger scripts and commands from any key combination
- Apply different rules per application

---

## Quick Start

**1. Install**

Download the latest binary for your platform from the
[Releases](https://github.com/AatozInnoInc/PC-Unifier/releases) page.

| Platform | Binary |
|---|---|
| Linux x86_64 | `pcunifier-linux-x86_64` |
| macOS ARM | `pcunifier-macos-aarch64` |
| macOS Intel | `pcunifier-macos-x86_64` |
| Windows x86_64 | `pcunifier-windows-x86_64.exe` |

**2. Create a config**

```toml
# ~/.config/pc-unifier/config.toml

[[remap]]
from = "Meta"
to   = "Ctrl"
```

**3. Run**

```sh
pcunifier
```

That is it. Key remapping is now active.

---

## Configuration

PC Unifier uses a TOML config file. The location depends on your OS:

| OS | Path |
|---|---|
| Linux | `~/.config/pc-unifier/config.toml` |
| macOS | `~/Library/Application Support/pc-unifier/config.toml` |
| Windows | `%APPDATA%\pc-unifier\config.toml` |

### Remap a key

```toml
[[remap]]
from = "CapsLock"
to   = "Escape"
```

### Remap only in specific apps

```toml
[[remap]]
from = "Meta"
to   = "Ctrl"
apps = ["org.mozilla.firefox", "org.gnome.Nautilus"]
```

Run `pcunifier --list-windows` to find the identifier for any open application.

### Create a hotkey

```toml
[[hotkey]]
keys    = ["Ctrl", "Alt", "T"]
action  = "exec"
command = "kitty"
```

### Expand a text snippet

```toml
[[hotstring]]
trigger     = ";;email"
replacement = "myemail@example.com"
```

See [docs/config-schema.md](docs/config-schema.md) for the full reference.

---

## Lua Scripting

For conditional logic and advanced automation, PC Unifier embeds a Lua scripting
engine (LuaJIT). Scripts run in microseconds and do not impact input latency.

```lua
-- ~/.config/pc-unifier/scripts/my_macros.lua

-- Different behavior per focused application
pcunifier.on_key("Meta", function(event)
    if pcunifier.focused_window().app_id == "org.mozilla.firefox" then
        return pcunifier.action.remap("Ctrl")
    end
    return pcunifier.action.passthrough()
end)
```

Load your script from config:

```toml
[[script]]
path = "~/.config/pc-unifier/scripts/my_macros.lua"
```

---

## Platform Requirements

### Linux

PC Unifier requires a Wayland compositor with support for the
`xdg-desktop-portal` Input Capture portal.

| Compositor | Minimum Version |
|---|---|
| KDE Plasma (KWin) | 5.27 |
| GNOME (Mutter) | 44 |
| Sway | 1.8 |
| Hyprland | 0.34 |

X11 sessions are supported via XRecord. Mixed XWayland environments are detected
automatically.

### macOS

PC Unifier requires Accessibility permission. On first run, you will be directed
to System Settings to grant access.

### Windows

No special permissions required. Run as a standard user.

---

## CLI Reference

```
pcunifier              Start the daemon
pcunifier --validate   Validate config and exit
pcunifier --reload     Send reload signal to running daemon
pcunifier --list-windows  Print identifiers for all open windows
pcunifier --version    Print version
pcunifier --help       Print help
```

---

## Building from Source

Requires: Rust 1.75+

```sh
git clone https://github.com/AatozInnoInc/PC-Unifier.git
cd PC-Unifier
cargo build --release
```

The binary is at `target/release/pcunifier`.

**Linux build dependencies:**

```sh
# Debian/Ubuntu
sudo apt install libei-dev libdbus-1-dev

# Fedora
sudo dnf install libei-devel dbus-devel

# Arch
sudo pacman -S libei dbus
```

---

## Documentation

| Document | Description |
|---|---|
| [docs/architecture.md](docs/architecture.md) | System design, module breakdown, platform backends |
| [docs/roadmap.md](docs/roadmap.md) | Development milestones and v1/v2 scope |
| [docs/config-schema.md](docs/config-schema.md) | Full config reference with all fields and examples |

---

## Contributing

PC Unifier is open source and welcomes contributions. Please open an issue before
starting work on a significant change.

---

## License

MIT
