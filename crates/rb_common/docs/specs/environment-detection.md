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