# Contributing Guide

Engineering standards for PC Unifier.

This document defines the coding standards, patterns, and practices that all contributors
(human and AI) must follow. These are non-negotiable.

## Table of Contents

1. [Philosophy](#philosophy)
2. [Rust Standards](#rust-standards)
3. [Code Organization](#code-organization)
4. [Error Handling](#error-handling)
5. [Testing](#testing)
6. [Documentation](#documentation)
7. [Git Workflow](#git-workflow)
8. [AI Model Handoff Protocol](#ai-model-handoff-protocol)

---

## Philosophy

### No Vibe Coding

This project demonstrates principal-level software engineering. Every decision should be:

- **Intentional**: Know why you are doing something
- **Documented**: Leave a trail for future maintainers
- **Tested**: If it is not tested, it is broken
- **Reviewed**: Code should be readable by humans

### Principles (Apply Judiciously)

Not all principles apply in all situations. Use judgment.

| Principle | When to Apply |
|---|---|
| SOLID | Trait design, module boundaries |
| DRY | When duplication creates maintenance burden, not before |
| YAGNI | Always. Do not build for hypothetical futures. |
| Composition over Inheritance | Rust has no inheritance. Prefer trait composition. |
| Fail Fast | Input validation, configuration parsing, platform detection |
| Explicit over Implicit | API design, configuration, error types |

---

## Rust Standards

### Edition and MSRV

All code targets **Rust 2021 edition**, MSRV 1.75+.

```toml
# Cargo.toml
[package]
edition = "2021"
rust-version = "1.75"
```

### Prefer Structs with Explicit Fields

```rust
// Good
pub struct ScenarioMetadata {
    pub id: String,
    pub name: String,
    pub severity: Severity,
}

// Avoid: tuple structs for non-trivial data
pub struct ScenarioMetadata(String, String, Severity);
```

### Use Enums for Variants and Discriminated Types

```rust
// Good: enum with data per variant
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    KeyDown {
        key: KeyCode,
        modifiers: Modifiers,
        window: Option<WindowContext>,
    },
    KeyUp {
        key: KeyCode,
        modifiers: Modifiers,
    },
    MouseMove {
        x: f64,
        y: f64,
    },
}

// Usage: exhaustive matching
fn handle_event(event: &TelemetryEvent) {
    match event {
        TelemetryEvent::KeyDown { key, modifiers, .. } => { /* ... */ }
        TelemetryEvent::KeyUp { key, .. } => { /* ... */ }
        TelemetryEvent::MouseMove { x, y } => { /* ... */ }
    }
}
```

### No `unwrap` or `expect` Outside Tests

`unwrap()` and `expect()` panic at runtime. They are only acceptable in test code
or in `main()` during early startup before the daemon loop begins.

```rust
// Never in library or engine code
let value = map.get("key").unwrap();

// Good: propagate with ?
let value = map.get("key").ok_or(ConfigError::MissingKey("key"))?;

// Good: provide a default
let value = map.get("key").unwrap_or(&default);
```

### Exhaustiveness in Match

Never use a catch-all `_` arm when matching on a project-owned enum. If a new
variant is added, the compiler must force every match site to handle it.

```rust
// Good: exhaustive, compiler will error if Severity gains a new variant
fn color_for_severity(s: Severity) -> &'static str {
    match s {
        Severity::Low      => "green",
        Severity::Medium   => "yellow",
        Severity::High     => "orange",
        Severity::Critical => "red",
    }
}

// Avoid: silent gap if new variant is added
fn color_for_severity(s: Severity) -> &'static str {
    match s {
        Severity::Critical => "red",
        _ => "green",
    }
}
```

### Use `#[derive]` Consistently

```rust
// Standard derives for data types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleId(String);

// Add Hash when used as map keys
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppId(pub String);

// Add Serialize/Deserialize at the boundary layer (config, IPC), not core types
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RemapConfig {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub apps: Vec<String>,
}
```

### Immutability by Default

Declare everything `let` (immutable) unless mutation is required. Use `mut` only
where necessary.

```rust
// Good
let rule_table = compile_rules(&config);

// Only when mutation is required
let mut buffer = InputBuffer::new();
buffer.push(event);
```

### Visibility

Keep items private by default. Export only what is part of the public API.

```rust
// Internal implementation detail
struct RuleCompiler { ... }

// Public API
pub struct RuleEngine { ... }
pub trait InputCapture { ... }
```

### Module and File Naming

- `snake_case` for all files and modules
- `mod.rs` only for module roots that re-export a public API
- Integration tests in `tests/` at the crate root
- Unit tests in the same file as the code under test

```
src/
  platform/
    mod.rs          # pub use linux::..., pub use macos::...
    linux/
      mod.rs
      capture.rs
      emulation.rs
    macos/
      mod.rs
      capture.rs
```

### Import (use) Order

```rust
// 1. Standard library
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crates
use mlua::prelude::*;
use serde::Deserialize;
use tokio::sync::mpsc;

// 3. Internal crate modules
use crate::config::RemapConfig;
use crate::platform::{InputCapture, ActionExecutor};

// 4. Super / self
use super::types::InputEvent;
```

---

## Code Organization

### One Concept Per File

```
// Good
rule_engine/
  mod.rs          # RuleEngine struct and public API
  compiler.rs     # compile_rules() - turns config into match table
  matcher.rs      # event matching logic
  types.rs        # Rule, Action, MatchResult

// Avoid
rule_engine.rs    # 600 lines with everything mixed together
```

### Explicit Public APIs via mod.rs

Each module has a `mod.rs` that explicitly controls what is exported:

```rust
// src/rule_engine/mod.rs

mod compiler;
mod matcher;
mod types;

pub use types::{Action, MatchResult, Rule};
pub use matcher::RuleEngine;
// compiler is internal - not re-exported
```

### Dependency Direction

```
main
  └── engine
        ├── config
        ├── rule_engine → event_bus → platform::traits
        └── lua_runtime
              └── platform::traits

platform::linux
platform::macos
platform::windows
  (implement platform::traits, do not import from engine)
```

Lower-level modules must not import from higher-level modules.

---

## Error Handling

### Use `thiserror` for Library Errors

```rust
use thiserror::Error;

// One error type per module boundary
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file not found at {path}")]
    NotFound { path: PathBuf },

    #[error("invalid config: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("unknown key name '{name}' at line {line}")]
    UnknownKey { name: String, line: usize },
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Accessibility permission not granted (macOS)")]
    AccessibilityPermissionDenied,

    #[error("xdg-desktop-portal Input Capture portal not available")]
    PortalUnavailable,

    #[error("input capture failed: {0}")]
    CaptureError(#[from] std::io::Error),
}
```

### Use `anyhow` Only in Binaries

`anyhow` is for top-level error reporting in `main.rs`. Library crates use typed
errors with `thiserror`.

```rust
// main.rs - acceptable
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config = load_config().context("failed to load config")?;
    run_daemon(config).context("daemon error")?;
    Ok(())
}

// lib.rs / any module - use typed errors
pub fn load_config(path: &Path) -> Result<Config, ConfigError> { ... }
```

### Error Chaining

```rust
// Good: preserve original error as source
fn connect_portal() -> Result<Portal, PlatformError> {
    dbus::connect()
        .map_err(|e| PlatformError::CaptureError(e.into()))
}
```

### Result Types for Expected Failures

For operations where failure is an expected outcome (not a bug), model it in the
return type rather than logging and returning a default.

```rust
// Good: caller decides how to handle each case
pub enum HotstringMatch {
    Complete(String),   // trigger matched, here is the replacement
    Partial,            // still accumulating
    NoMatch,            // buffer does not match any trigger
}

pub fn check_buffer(buffer: &str, rules: &[HotstringRule]) -> HotstringMatch { ... }
```

---

## Testing

### Test Location

Unit tests live in the same file as the code under test. Integration tests live in
`tests/` at the crate root.

```
src/
  rule_engine/
    matcher.rs          # #[cfg(test)] mod tests { ... } at bottom
tests/
  remap_integration.rs  # tests against the full engine
  hotkey_integration.rs
```

### Test Structure

Use Arrange-Act-Assert with clear variable names:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remap_rule_suppresses_source_key_and_injects_target() {
        // Arrange
        let rules = vec![Rule::Remap {
            from: KeyCode::Meta,
            to: KeyCode::Ctrl,
            apps: vec![],
        }];
        let engine = RuleEngine::new(rules);
        let event = InputEvent::KeyDown {
            key: KeyCode::Meta,
            modifiers: Modifiers::empty(),
            window: None,
        };

        // Act
        let result = engine.process(&event);

        // Assert
        assert_eq!(result, MatchResult::Remap(KeyCode::Ctrl));
    }

    #[test]
    fn remap_rule_passes_through_non_matching_key() {
        // Arrange
        let rules = vec![Rule::Remap {
            from: KeyCode::Meta,
            to: KeyCode::Ctrl,
            apps: vec![],
        }];
        let engine = RuleEngine::new(rules);
        let event = InputEvent::KeyDown {
            key: KeyCode::A,
            modifiers: Modifiers::empty(),
            window: None,
        };

        // Act
        let result = engine.process(&event);

        // Assert
        assert_eq!(result, MatchResult::Passthrough);
    }
}
```

### Test Naming

Name tests as a description of behavior, not the function under test.

```rust
// Good
#[test] fn remap_rule_activates_only_in_matching_app()
#[test] fn hotstring_expands_on_complete_trigger_match()
#[test] fn config_error_includes_line_number_on_invalid_key()
#[test] fn event_bus_does_not_drop_events_under_load()

// Avoid
#[test] fn test_remap()
#[test] fn it_works()
#[test] fn edge_case()
```

### Async Tests

Use `tokio::test` for async code:

```rust
#[tokio::test]
async fn event_bus_delivers_ten_thousand_events_without_loss() {
    // Arrange
    let (tx, mut rx) = mpsc::channel(1024);
    let expected = 10_000usize;

    // Act
    for i in 0..expected {
        tx.send(InputEvent::test_keydown(i as u8)).await.unwrap();
    }
    drop(tx);

    let mut received = 0usize;
    while rx.recv().await.is_some() {
        received += 1;
    }

    // Assert
    assert_eq!(received, expected);
}
```

### Coverage Requirements

- Minimum 80% line coverage for `pc-unifier-core`
- All public API functions must have tests
- All error paths must have tests
- Run with: `cargo llvm-cov --all-features`

---

## Documentation

### Doc Comments for Public APIs

```rust
/// Evaluates a single input event against the compiled rule table.
///
/// Returns the highest-priority matching action, or `MatchResult::Passthrough`
/// if no rule matches.
///
/// # Performance
///
/// This function is called on every input event. It must not allocate
/// or perform I/O. Current complexity is O(n) over the rule table.
///
/// # Example
///
/// ```rust
/// let engine = RuleEngine::new(rules);
/// let result = engine.process(&event);
/// match result {
///     MatchResult::Remap(key) => executor.inject_key(key),
///     MatchResult::Passthrough => { /* forward original event */ }
///     MatchResult::Suppress => { /* discard event */ }
/// }
/// ```
pub fn process(&self, event: &InputEvent) -> MatchResult { ... }
```

### Architecture Decision Records

Major decisions get an ADR in `docs/decisions/`:

```markdown
# ADR-001: Use Rust

## Status
Accepted

## Context
We need a single binary that runs as a system-level input daemon on Linux, macOS,
and Windows. The daemon intercepts all keyboard and mouse input with a hard latency
budget of 33ms end-to-end.

## Decision
Use Rust 2021, MSRV 1.75. The compiler enforces memory safety in a context where
bugs can destabilize a desktop session. The crate ecosystem (mlua, tokio, thiserror,
serde) covers all required functionality.

## Consequences
- Memory safety enforced at compile time, not by convention
- Steeper learning curve than C++ for contributors unfamiliar with Rust
- Single static binary with no runtime dependencies
```

---

## Git Workflow

### Branch Naming

```
feature/rule-engine-hotstrings
fix/linux-portal-fallback-detection
docs/update-config-schema
refactor/event-bus-backpressure
```

### Commit Messages

Follow Conventional Commits:

```
feat(rule-engine): add hotstring expansion with rolling input buffer

fix(linux): handle XWayland fallback when portal is unavailable

docs: add ADR-002 for Lua scripting engine choice

test(config): add coverage for unknown key name error path

refactor(platform): extract InputCapture trait to platform/mod.rs
```

### PR Requirements

1. All CI checks pass (`cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`)
2. Coverage does not decrease
3. Public API changes have doc comment updates
4. Breaking changes noted in PR description

---

## AI Model Handoff Protocol

This project uses AI models for design, implementation, and review. Each has a role:

| Model | Role | Capabilities |
|-------|------|--------------|
| Opus 4.6 | Design, architecture, review | Big-picture thinking, conceptual integrity |
| GPT-5.2 Codex | Implementation to spec | Precise code generation, test writing |
| Sonnet 4.6 | Quick iterations, glue code | Fast turnaround, minor fixes |


### Handoff Document Format

When completing a work session, create or update `HANDOFF.md`:

```markdown
# Handoff: [Date] [From] to [To]

## Completed
- Implemented `RuleEngine` in `src/rule_engine/matcher.rs`
- Added unit tests (92% coverage)
- Compiler pass: `cargo clippy` clean, `cargo fmt` applied

## Next Tasks
1. Implement `ConfigParser` in `src/config/`
   - Must deserialize all four block types (remap, hotkey, hotstring, script)
   - See docs/config-schema.md for full field reference
2. Add error messages for all validation failure cases
   - See docs/architecture.md - Error Messages section

## Constraints
- DO NOT modify `InputEvent` or `Action` enums without design review
- All new code must have tests before the PR is opened
- Follow CONTRIBUTING.md strictly

## Open Questions
- Should hotstring matching be case-sensitive by default? (Deferred for design review)

## Files Modified
- src/rule_engine/matcher.rs (new)
- src/rule_engine/types.rs (modified)
- src/rule_engine/matcher_tests.rs (new)

## Prompt for Next Agent
[A tailored prompt for the receiving agent. Include full context, constraints,
and which files to read first.]
```

### Rules for All Models

1. Read `CLAUDE.md`, `docs/architecture.md`, and `CONTRIBUTING.md` before starting work.
2. Do not deviate from the design without explicit approval.
3. Write tests for all new code before opening a PR.
4. Update `HANDOFF.md` when completing a session.
5. Ask for design review if requirements are unclear.
6. Follow coding standards exactly.
7. Include a tailored "Prompt for Next Agent" section in `HANDOFF.md`.

---

## Appendix: Tooling

### Clippy Configuration

```toml
# .cargo/config.toml
[target.'cfg(all())']
rustflags = [
    "-W", "clippy::all",
    "-W", "clippy::pedantic",
    "-W", "clippy::unwrap_used",       # no unwrap in library code
    "-W", "clippy::expect_used",       # no expect in library code
    "-W", "clippy::panic",             # no panic in library code
    "-A", "clippy::module_name_repetitions",  # allowed: InputCapture in input module
]
```

### Rustfmt Configuration

```toml
# rustfmt.toml
edition = "2021"
max_width = 100
tab_spaces = 4
trailing_comma = "Vertical"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

### CI Configuration

```yaml
# .github/workflows/ci.yml (abbreviated)
jobs:
  check:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo build --release
      - run: cargo test
```

### Recommended Crates

| Purpose | Crate |
|---|---|
| Async runtime | `tokio` |
| Lua scripting | `mlua` (LuaJIT feature) |
| Config parsing | `serde` + `toml` |
| Error types | `thiserror` |
| Binary error handling | `anyhow` |
| Linux input | `reis` (libei bindings) |
| Linux portal | `ashpd` (xdg-desktop-portal) |
| macOS input | `core-graphics` |
| Windows input | `windows-rs` |
| Logging | `tracing` + `tracing-subscriber` |
| Coverage | `cargo-llvm-cov` |

---

Document version: 1.0.0
Last updated: 2026-02-19
