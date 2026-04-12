// ── ColorPreference ───────────────────────────────────────────────────────────

/// Explicit caller override for color emission.
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

// ── EnvironmentSnapshot ───────────────────────────────────────────────────────

/// A point-in-time capture of environment facts relevant to output decisions.
///
/// Callers should create one snapshot and pass it to renderers and sinks rather
/// than re-probing the environment per diagnostic.
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
    /// Terminal column width, if determinable.
    pub width: Option<usize>,
    /// Explicit per-stream color override.
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

    /// Returns the effective output width in columns (default: 80).
    pub fn effective_width(&self) -> usize {
        self.width.unwrap_or(80)
    }
}

// ── EnvironmentDetector trait ─────────────────────────────────────────────────

/// Abstracts environment probing so tests can inject deterministic states
/// without touching real environment variables or file descriptors.
pub trait EnvironmentDetector {
    /// Probes the environment and returns a snapshot of the current state.
    fn detect(&self) -> EnvironmentSnapshot;
}

// ── RealEnvironmentDetector ───────────────────────────────────────────────────

/// The production implementation. Reads live process environment.
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
        // Width is populated by the consuming crate behind a terminal-size feature.
        let width = None;

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

// ── FixedEnvironmentDetector ──────────────────────────────────────────────────

/// Test helper. Creates a fully controlled snapshot without probing the real
/// environment.
pub struct FixedEnvironmentDetector {
    /// The snapshot that will be returned by every call to [`EnvironmentDetector::detect`].
    pub snapshot: EnvironmentSnapshot,
}

impl FixedEnvironmentDetector {
    /// Creates a snapshot that simulates a colour-capable terminal (stdout + stderr are TTY).
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

    /// Creates a snapshot that simulates a CI environment with redirected output (no TTY).
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

    /// Creates a snapshot that simulates a terminal with `NO_COLOR` set.
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
