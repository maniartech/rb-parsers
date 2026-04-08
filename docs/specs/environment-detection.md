# Spec: Environment Detection

**Status**: Ready for implementation
**Module**: `rb_common::env`
**Depends on**: nothing
**Requirement source**: `docs/requirements/environment-detection.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Color precedence | `CLICOLOR_FORCE` → `NO_COLOR` → `CLICOLOR` → TTY detection → CI fallback |
| Width fallback | 80 columns when terminal size is unavailable |
| Global state | No hidden globals; detection returns an owned `EnvironmentSnapshot` |
| TTY detection API | Delegate to `std::io::IsTerminal` (stable since Rust 1.70) |

---

## Module Layout

```
rb_common::env
├── ColorPreference
├── EnvironmentSnapshot
├── EnvironmentDetector  (trait)
├── RealEnvironmentDetector
└── FixedEnvironmentDetector  (for testing)
```

---

## Types

### ColorPreference

Explicit caller override for color emission.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorPreference {
    /// Detect from environment; emit color when the target stream is a
    /// TTY and no suppression variable is set.
    #[default]
    Auto,
    /// Always emit color sequences regardless of environment.
    Always,
    /// Never emit color sequences regardless of environment.
    Never,
}
```

---

### EnvironmentSnapshot

A point-in-time capture of environment facts relevant to output decisions.
Callers should create one snapshot and pass it to renderers and sinks rather
than re-probing the environment per diagnostic.

```rust
#[derive(Debug, Clone)]
pub struct EnvironmentSnapshot {
    /// Whether stdout is a TTY at snapshot time.
    pub stdout_is_tty: bool,
    /// Whether stderr is a TTY at snapshot time.
    pub stderr_is_tty: bool,
    /// `true` when the `NO_COLOR` environment variable is set to a non-empty value.
    pub no_color: bool,
    /// Raw value of `CLICOLOR` if set.
    pub clicolor: Option<String>,
    /// Raw value of `CLICOLOR_FORCE` if set.
    pub clicolor_force: Option<String>,
    /// `true` when a known CI environment variable such as `CI` is set.
    pub ci: bool,
    /// Raw value of the `TERM` variable if set.
    pub term: Option<String>,
    /// Terminal column width, if determinable. `None` when the query failed
    /// or the stream is not a TTY.
    pub width: Option<usize>,
    /// Explicit per-stream color override. When `Some`, this takes precedence
    /// over all environment variable heuristics.
    pub color_override: Option<ColorPreference>,
}

impl EnvironmentSnapshot {
    /// Resolves the effective color behavior for a given target stream.
    ///
    /// Precedence:
    /// 1. `color_override` field (explicit caller instruction)
    /// 2. `CLICOLOR_FORCE` → force on
    /// 3. `NO_COLOR` → force off
    /// 4. `CLICOLOR=0` → force off
    /// 5. TTY detection (on when TTY, off when redirected)
    /// 6. Off in known CI environments without explicit override
    pub fn effective_color(&self, target_is_tty: bool) -> bool {
        if let Some(pref) = self.color_override {
            return match pref {
                ColorPreference::Always => true,
                ColorPreference::Never => false,
                ColorPreference::Auto => self.auto_color(target_is_tty),
            };
        }
        self.auto_color(target_is_tty)
    }

    fn auto_color(&self, target_is_tty: bool) -> bool {
        // CLICOLOR_FORCE overrides everything when set to non-"0"
        if let Some(ref v) = self.clicolor_force {
            if v != "0" {
                return true;
            }
        }
        // NO_COLOR suppresses color
        if self.no_color {
            return false;
        }
        // CLICOLOR=0 suppresses color
        if self.clicolor.as_deref() == Some("0") {
            return false;
        }
        // Fall through to TTY detection
        target_is_tty
    }

    /// Returns the effective output width in columns.
    ///
    /// Priority: detected terminal width → explicit override → 80 column default.
    pub fn effective_width(&self) -> usize {
        self.width.unwrap_or(80)
    }
}
```

---

### EnvironmentDetector (trait)

Abstracts environment probing so tests can inject deterministic states without
touching real environment variables or file descriptors.

```rust
pub trait EnvironmentDetector {
    fn detect(&self) -> EnvironmentSnapshot;
}
```

---

### RealEnvironmentDetector

The production implementation. Reads live process environment.

```rust
pub struct RealEnvironmentDetector;

impl EnvironmentDetector for RealEnvironmentDetector {
    fn detect(&self) -> EnvironmentSnapshot {
        use std::io::IsTerminal;
        use std::env;

        let stdout_is_tty = std::io::stdout().is_terminal();
        let stderr_is_tty = std::io::stderr().is_terminal();
        let no_color = env::var("NO_COLOR").map(|v| !v.is_empty()).unwrap_or(false);
        let clicolor = env::var("CLICOLOR").ok();
        let clicolor_force = env::var("CLICOLOR_FORCE").ok();
        let ci = env::var("CI").is_ok();
        let term = env::var("TERM").ok();

        // Best-effort width query via terminal size; library code should not
        // depend on a specific terminal-size crate being present, so the
        // implementation crate can gate this behind a feature flag.
        let width = None; // populated by concrete crate using e.g. `terminal_size` crate

        EnvironmentSnapshot {
            stdout_is_tty,
            stderr_is_tty,
            no_color,
            clicolor,
            clicolor_force,
            ci,
            term,
            width,
            color_override: None,
        }
    }
}
```

---

### FixedEnvironmentDetector

Test helper. Creates a fully controlled snapshot without probing the real
environment.

```rust
pub struct FixedEnvironmentDetector {
    pub snapshot: EnvironmentSnapshot,
}

impl FixedEnvironmentDetector {
    pub fn plain_terminal() -> Self {
        FixedEnvironmentDetector {
            snapshot: EnvironmentSnapshot {
                stdout_is_tty: true,
                stderr_is_tty: true,
                no_color: false,
                clicolor: None,
                clicolor_force: None,
                ci: false,
                term: Some("xterm-256color".into()),
                width: Some(120),
                color_override: None,
            },
        }
    }

    pub fn redirected_ci() -> Self {
        FixedEnvironmentDetector {
            snapshot: EnvironmentSnapshot {
                stdout_is_tty: false,
                stderr_is_tty: false,
                no_color: false,
                clicolor: None,
                clicolor_force: None,
                ci: true,
                term: None,
                width: None,
                color_override: None,
            },
        }
    }

    pub fn no_color() -> Self {
        FixedEnvironmentDetector {
            snapshot: EnvironmentSnapshot {
                stdout_is_tty: true,
                stderr_is_tty: true,
                no_color: true,
                clicolor: None,
                clicolor_force: None,
                ci: false,
                term: Some("xterm-256color".into()),
                width: Some(80),
                color_override: None,
            },
        }
    }
}

impl EnvironmentDetector for FixedEnvironmentDetector {
    fn detect(&self) -> EnvironmentSnapshot {
        self.snapshot.clone()
    }
}
```

---

## Implementation Notes

- `RealEnvironmentDetector` leaves `width: None` intentionally. The implementing
  crate that adds a terminal-size dependency fills this field behind a
  `terminal-size` or similar Cargo feature flag. Core logic must never hard-depend
  on a terminal-size crate.
- Tests must always use `FixedEnvironmentDetector` so they are not sensitive to
  the CI or developer machine environment.
- `EnvironmentSnapshot` is `Clone` so it can be cheaply passed into sinks,
  renderers, and renderer-selection logic without shared references.
- No `lazy_static` or `once_cell` globals are used. Callers control snapshot
  lifetime.
# Environment Detection

## Objective

Define how `rb_common` detects terminal and host-environment capabilities that affect diagnostics output.

This spec covers:

- color policy
- terminal versus redirected output
- width detection
- environment-variable overrides
- deterministic testing and explicit overrides

## Why This Needs Its Own Spec

The crate goals already include terminal rendering and non-color behavior. Those decisions depend on environment detection, but environment detection should not be scattered across renderers or low-level libraries.

The design must stay:

- overridable by the caller
- deterministic in tests
- free from hidden global behavior where possible

## Core Principle

Environment detection should produce facts and preferences, not directly emit diagnostics or render output.

That means:

- renderers consume an environment snapshot or resolved policy
- libraries can override detection explicitly
- tests can inject fixed conditions without depending on the host terminal

Environment detection should inform renderer selection. It should not itself decide by side effect which renderer runs.

In the common case, callers should be able to rely on automatic detection without having to manually configure every environment detail.

## Likely Types

```rust
pub enum ColorPreference {
    Auto,
    Always,
    Never,
}

pub struct EnvironmentSnapshot {
    pub stdout_is_tty: bool,
    pub stderr_is_tty: bool,
    pub no_color: bool,
    pub clicolor: Option<String>,
    pub clicolor_force: Option<String>,
    pub ci: bool,
    pub term: Option<String>,
    pub width: Option<usize>,
}
```

The exact fields may change, but the idea is to isolate host detection from rendering policy.

## Requirements

1. Callers must be able to bypass environment detection entirely with explicit configuration.
2. Detection must be testable without depending on the process environment.
3. Color behavior must not rely on one platform-specific heuristic.
4. Width detection must have a safe fallback when terminal size is unavailable.
5. Environment detection must not force output to stderr or stdout by itself.
6. Environment detection must provide structured inputs for renderer selection rather than embedding renderer-specific policy branches.
7. Automatic detection must be good enough to serve as the default behavior for most users.
8. Every automatic decision that materially affects output must remain overrideable.

## Color Policy

The minimum supported model should be:

- `Always`: emit color regardless of environment
- `Never`: emit no color regardless of environment
- `Auto`: detect based on environment facts

Likely influences on `Auto` include:

- `NO_COLOR`
- `CLICOLOR`
- `CLICOLOR_FORCE`
- TTY detection
- known CI environments

The precedence rules should be explicit rather than implicit.

## Width Detection

Renderers need an effective width, but width should be optional input rather than a mandatory global query.

Suggested behavior:

1. explicit caller-provided width wins
2. environment or terminal query is used when available
3. renderer falls back to a conservative default when width is unknown

## TTY and Stream Awareness

Diagnostics may be written to stdout, stderr, files, or in-memory buffers.

The environment model should allow callers to express which stream is being targeted so the renderer does not make incorrect assumptions about interactive behavior.

This is important because renderer selection should be based on the target stream and environment facts, not on whichever renderer checks first.

## Testability

The environment layer should be easy to fake.

Tests should be able to assert:

- color disabled by policy
- color forced on
- width fallback behavior
- plain output selected for redirected streams

This is important because diagnostics behavior should not vary unpredictably across developer machines and CI agents.

## Configuration Guidance

Best practice is:

1. make `Auto` the default for common policies like color and format selection
2. allow explicit overrides to short-circuit auto-detection
3. keep raw environment facts available for advanced consumers and testing

This preserves good out-of-the-box behavior while still allowing precise control when embedding the framework into editors, browsers, CI, or custom tooling.

## Open Questions

1. Should CI default to plain output unless color is explicitly forced?
2. Should width fallback live in environment detection or renderer configuration?
3. Should environment detection expose raw variables, resolved policy, or both?
4. How much platform-specific terminal handling should live in `rb_common` versus a companion utility module?