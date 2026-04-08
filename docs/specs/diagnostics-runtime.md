# Spec: Diagnostics Runtime

**Status**: Ready for implementation
**Module**: `rb_common::diagnostics`
**Depends on**: `rb_common::spans`, `rb_common::catalog`, `rb_common::suggestions`
**Requirement source**: `docs/requirements/diagnostics-runtime.md`, `docs/requirements/error-system.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Interior state vs explicit returns | `DiagnosticsContext` is passed as `&mut DiagnosticsContext`; no hidden interior mutability in the normal path. Thread-safe sharing is via the caller wrapping in `Arc<Mutex<...>>` when needed. |
| Sink hook receiver | Sinks receive `&Diagnostic` (immutable, already finalized). No pre-finalization builder API. |
| Diagnostic ordering | Strictly emission order. Severity grouping is a renderer concern; the runtime preserves insertion order. |
| Suppression and severity remapping | Belongs to `SeverityPolicy`, a separate configuration struct layered above the runtime. Not part of the core `DiagnosticsContext`. |
| Deduplication | **None in Phase 1.** `DiagnosticsContext` does not deduplicate diagnostics. Exact-duplicate suppression is deferred to Phase 2 when the parser can produce cascading identical errors. Implementing deduplication prematurely would hide real emission bugs during early development. When added, it will be an explicit `DeduplicationMode` field on `DiagnosticsContext`, defaulting to `Off`. |
| Subsystem filter | **Not provided in Phase 1.** `DiagnosticsContext` has no subsystem or namespace filter. `by_code()` is sufficient for targeted lookup. A subsystem filter would require callers to reason about namespace strings at the wrong level. If a crate needs its own diagnostic slice it should maintain its own `DiagnosticsContext`. |

---

## Module Layout

```
rb_common::diagnostics
├── DiagnosticSeverity     (re-exported from catalog::ErrorSeverity alias)
├── Hint
├── Diagnostic
├── DiagnosticBuilder
├── DiagnosticsMode
├── DiagnosticSink (trait)
├── NullSink
├── CollectingSink
├── HookSink
├── CompositeSink
├── DiagnosticsContext
└── SeverityPolicy
```

---

## Types

### Hint

A short, actionable prose string attached to a `Diagnostic`.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    /// The hint text. Must be specific and actionable.
    pub text: String,
    /// Whether this hint was generated automatically by the framework or
    /// explicitly authored by the grammar/language author.
    pub is_auto: bool,
}

impl Hint {
    pub fn authored(text: impl Into<String>) -> Self {
        Hint { text: text.into(), is_auto: false }
    }

    pub fn auto_generated(text: impl Into<String>) -> Self {
        Hint { text: text.into(), is_auto: true }
    }
}
```

---

### Diagnostic

The complete representation of one emitted diagnostic. This is the central
type that sinks, renderers, and tooling consumers all work with.

```rust
use crate::catalog::{ErrorCode, ErrorSeverity};
use crate::spans::{DiagnosticLocation, SpanLabel, DiagnosticContextRegion};
use crate::suggestions::Suggestion;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// The stable error code from the owning catalog.
    pub code: ErrorCode,
    /// Effective severity after any policy remapping.
    pub severity: ErrorSeverity,
    /// Short human-readable title (from the template).
    pub title: &'static str,
    /// The rendered message, with any placeholder substitutions applied.
    pub message: String,
    /// The primary failure location and any secondary/context labels.
    pub labels: Vec<SpanLabel>,
    /// Optional enclosing-scope hierarchy for context rendering.
    pub context: Option<DiagnosticContextRegion>,
    /// Additional factual notes (not actionable).
    pub notes: Vec<String>,
    /// Actionable hints.
    pub hints: Vec<Hint>,
    /// Structured fixes, ordered: primary first, then by applicability.
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    pub fn primary_location(&self) -> Option<&DiagnosticLocation> {
        use crate::spans::LabelStyle;
        self.labels
            .iter()
            .find(|l| l.style == LabelStyle::Primary)
            .map(|l| &l.location)
    }

    pub fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }
}
```

---

### DiagnosticBuilder

Ergonomic builder for constructing a `Diagnostic` from a template code.
The pattern avoids stringly-typed construction.

```rust
use crate::catalog::{ErrorCatalog, ErrorCode, ErrorSeverity};
use crate::spans::{DiagnosticLocation, SpanLabel, LabelStyle, DiagnosticContextRegion};
use crate::suggestions::Suggestion;

pub struct DiagnosticBuilder {
    inner: Diagnostic,
}

impl DiagnosticBuilder {
    /// Starts a builder from a known template. `message` is the rendered
    /// message with placeholders already substituted.
    pub fn from_template(
        catalog: &dyn ErrorCatalog,
        code: ErrorCode,
        message: impl Into<String>,
    ) -> Self {
        let template = catalog.get(code).expect("unknown error code");
        DiagnosticBuilder {
            inner: Diagnostic {
                code,
                severity: template.severity,
                title: template.title,
                message: message.into(),
                labels: Vec::new(),
                context: None,
                notes: Vec::new(),
                hints: template
                    .default_hints
                    .iter()
                    .map(|h| Hint::authored(*h))
                    .collect(),
                suggestions: Vec::new(),
            },
        }
    }

    pub fn primary(mut self, location: DiagnosticLocation) -> Self {
        self.inner.labels.insert(0, SpanLabel::primary(location));
        self
    }

    pub fn primary_labeled(mut self, location: DiagnosticLocation, msg: impl Into<String>) -> Self {
        self.inner.labels.insert(0, SpanLabel::primary_with_message(location, msg));
        self
    }

    pub fn secondary(mut self, location: DiagnosticLocation) -> Self {
        self.inner.labels.push(SpanLabel::secondary(location));
        self
    }

    pub fn secondary_labeled(mut self, location: DiagnosticLocation, msg: impl Into<String>) -> Self {
        self.inner.labels.push(SpanLabel::secondary_with_message(location, msg));
        self
    }

    pub fn context(mut self, region: DiagnosticContextRegion) -> Self {
        self.inner.context = Some(region);
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.inner.notes.push(note.into());
        self
    }

    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.inner.hints.push(Hint::authored(text));
        self
    }

    pub fn suggestion(mut self, suggestion: Suggestion) -> Self {
        self.inner.suggestions.push(suggestion);
        self
    }

    pub fn severity(mut self, severity: ErrorSeverity) -> Self {
        self.inner.severity = severity;
        self
    }

    pub fn build(self) -> Diagnostic {
        self.inner
    }
}
```

---

### DiagnosticsMode

Controls whether a `DiagnosticsContext` collects, emits, both, or neither.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagnosticsMode {
    /// Ignore all diagnostics. Useful when the caller does not care about errors.
    Disabled,
    /// Store diagnostics internally for later inspection without any output.
    #[default]
    Collect,
    /// Emit through sinks immediately; do not retain.
    Emit,
    /// Both collect and emit.
    CollectAndEmit,
}
```

---

### DiagnosticSink (trait)

The output interface. Each sink owns exactly one output path.

```rust
pub trait DiagnosticSink: Send + Sync {
    /// Called once per finalized diagnostic. The diagnostic is immutable;
    /// the sink may format and write it, store it, or forward it.
    fn emit(&self, diagnostic: &Diagnostic);
}
```

---

### NullSink

Discards all diagnostics.

```rust
pub struct NullSink;

impl DiagnosticSink for NullSink {
    fn emit(&self, _diagnostic: &Diagnostic) {}
}
```

---

### CollectingSink

Stores diagnostics for later inspection. Uses `Mutex` for `Send + Sync`.

```rust
use std::sync::Mutex;

pub struct CollectingSink {
    collected: Mutex<Vec<Diagnostic>>,
}

impl CollectingSink {
    pub fn new() -> Self {
        CollectingSink { collected: Mutex::new(Vec::new()) }
    }

    pub fn take_all(&self) -> Vec<Diagnostic> {
        std::mem::take(&mut *self.collected.lock().unwrap())
    }

    pub fn snapshot(&self) -> Vec<Diagnostic> {
        self.collected.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.collected.lock().unwrap().len()
    }

    pub fn error_count(&self) -> usize {
        use crate::catalog::ErrorSeverity;
        self.collected
            .lock()
            .unwrap()
            .iter()
            .filter(|d| d.severity == ErrorSeverity::Error)
            .count()
    }
}

impl DiagnosticSink for CollectingSink {
    fn emit(&self, diagnostic: &Diagnostic) {
        self.collected.lock().unwrap().push(diagnostic.clone());
    }
}
```

---

### HookSink

Calls a user-provided callback for each diagnostic. Useful for integration
points that need custom routing or inspection.

```rust
pub struct HookSink<F: Fn(&Diagnostic) + Send + Sync> {
    hook: F,
}

impl<F: Fn(&Diagnostic) + Send + Sync> HookSink<F> {
    pub fn new(hook: F) -> Self {
        HookSink { hook }
    }
}

impl<F: Fn(&Diagnostic) + Send + Sync> DiagnosticSink for HookSink<F> {
    fn emit(&self, diagnostic: &Diagnostic) {
        (self.hook)(diagnostic);
    }
}
```

---

### CompositeSink

Fan-out to multiple sinks. Each child receives the same diagnostic in order.

```rust
pub struct CompositeSink {
    sinks: Vec<Box<dyn DiagnosticSink>>,
}

impl CompositeSink {
    pub fn new() -> Self {
        CompositeSink { sinks: Vec::new() }
    }

    pub fn add(mut self, sink: Box<dyn DiagnosticSink>) -> Self {
        self.sinks.push(sink);
        self
    }
}

impl DiagnosticSink for CompositeSink {
    fn emit(&self, diagnostic: &Diagnostic) {
        for sink in &self.sinks {
            sink.emit(diagnostic);
        }
    }
}
```

---

### DiagnosticsContext

The per-pipeline diagnostics state. Passed as `&mut DiagnosticsContext` to
tokenizer and parser internals. Thread-safe sharing is the caller's
responsibility (wrap in `Arc<Mutex<...>>` when sharing across threads).

```rust
pub struct DiagnosticsContext {
    mode: DiagnosticsMode,
    sink: Option<Box<dyn DiagnosticSink>>,
    collected: Vec<Diagnostic>,
    error_count: usize,
    warning_count: usize,
}

impl DiagnosticsContext {
    /// Creates a collecting-only context. No output until `take_all()` is called.
    pub fn collecting() -> Self {
        DiagnosticsContext {
            mode: DiagnosticsMode::Collect,
            sink: None,
            collected: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Creates a context that emits through the given sink and does not retain.
    pub fn emitting(sink: Box<dyn DiagnosticSink>) -> Self {
        DiagnosticsContext {
            mode: DiagnosticsMode::Emit,
            sink: Some(sink),
            collected: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Creates a context that both collects and emits.
    pub fn collecting_and_emitting(sink: Box<dyn DiagnosticSink>) -> Self {
        DiagnosticsContext {
            mode: DiagnosticsMode::CollectAndEmit,
            sink: Some(sink),
            collected: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Disabled context: ignores all diagnostics.
    pub fn null() -> Self {
        DiagnosticsContext {
            mode: DiagnosticsMode::Disabled,
            sink: None,
            collected: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Emits one finalized diagnostic. Ordering is strictly emission order.
    pub fn emit(&mut self, diagnostic: Diagnostic) {
        use crate::catalog::ErrorSeverity;
        match diagnostic.severity {
            ErrorSeverity::Error => self.error_count += 1,
            ErrorSeverity::Warning => self.warning_count += 1,
            _ => {}
        }
        if matches!(self.mode, DiagnosticsMode::Collect | DiagnosticsMode::CollectAndEmit) {
            self.collected.push(diagnostic.clone());
        }
        if matches!(self.mode, DiagnosticsMode::Emit | DiagnosticsMode::CollectAndEmit) {
            if let Some(sink) = &self.sink {
                sink.emit(&diagnostic);
            }
        }
    }

    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    pub fn error_count(&self) -> usize {
        self.error_count
    }

    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    /// Returns all collected diagnostics, preserving emission order.
    pub fn collected(&self) -> &[Diagnostic] {
        &self.collected
    }

    /// Removes and returns all collected diagnostics.
    pub fn take_all(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.collected)
    }

    /// Filters collected diagnostics by severity.
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        use crate::catalog::ErrorSeverity;
        self.collected.iter().filter(|d| d.severity == ErrorSeverity::Error)
    }

    /// Filters collected diagnostics by error code.
    pub fn by_code(&self, code: ErrorCode) -> impl Iterator<Item = &Diagnostic> {
        self.collected.iter().filter(move |d| d.code == code)
    }
}
```

---

### SeverityPolicy

Optional severity remapping and code suppression. Applied by the caller before
constructing a `Diagnostic` or before passing it to a `DiagnosticsContext`.
This keeps the runtime itself unopinionated about policy.

```rust
use crate::catalog::{ErrorCode, ErrorSeverity};

#[derive(Debug, Clone, Default)]
pub struct SeverityPolicy {
    /// Treat warnings as errors.
    pub warnings_as_errors: bool,
    /// Suppressed error codes. Diagnostics with these codes are not emitted.
    pub suppressed: Vec<ErrorCode>,
    /// Per-code severity overrides.
    pub overrides: Vec<(ErrorCode, ErrorSeverity)>,
}

impl SeverityPolicy {
    pub fn apply(&self, mut diagnostic: Diagnostic) -> Option<Diagnostic> {
        // Check suppression
        if self.suppressed.contains(&diagnostic.code) {
            return None;
        }
        // Apply per-code override
        if let Some((_, sev)) = self.overrides.iter().find(|(c, _)| *c == diagnostic.code) {
            diagnostic.severity = *sev;
        }
        // Warnings as errors
        if self.warnings_as_errors && diagnostic.severity == ErrorSeverity::Warning {
            diagnostic.severity = ErrorSeverity::Error;
        }
        Some(diagnostic)
    }
}
```

---

## Integration Pattern

Typical pipeline usage:

```rust
// 1. Create the context
let mut ctx = DiagnosticsContext::collecting();

// 2. Run tokenizer and parser, both take &mut ctx
let tokens = tokenizer.tokenize(source, &mut ctx);
let tree = parser.parse(&tokens, &mut ctx);

// 3. Inspect results
if ctx.has_errors() {
    for d in ctx.errors() {
        // render or return
    }
}
```

For richer output, swap to `collecting_and_emitting`:

```rust
let sink = Box::new(CollectingSink::new());
let mut ctx = DiagnosticsContext::collecting_and_emitting(sink);
```

---

## Implementation Notes

- `DiagnosticsContext` is intentionally not `Clone`; it owns the diagnostic
  stream. Sharing across threads means the caller wraps it in `Arc<Mutex<...>>`.
- The `emit()` method takes an owned `Diagnostic`. Builders should finalize
  before handing off.
- For tokenizer/parser integration, both layers must accept `&mut DiagnosticsContext`
  rather than owning a private context, so they share one coherent stream.
- `SeverityPolicy` is applied at the call site (before or instead of `ctx.emit()`),
  not inside the context, keeping policy separate from collection.
# Diagnostics Runtime

## Objective

Define how diagnostics are emitted, collected, ordered, deduplicated, and shared across tokenizer and parser instances.

This spec covers runtime behavior, not diagnostic content.

Recovery and resume policy is defined in `recovery-and-error-boundaries.md`, but the runtime must be able to collect diagnostics emitted during those workflows coherently.

## Core Principle

Diagnostics must be instance-scoped, not process-global.

That means:

- one tokenizer can collect diagnostics silently
- another tokenizer can emit to a terminal sink
- a parser can share the same diagnostics context as the tokenizer that produced its tokens
- tests can inspect diagnostics deterministically without global side effects

The runtime should offer good defaults so most consumers can get high-quality diagnostics without assembling a complex sink graph by hand.

## Core Types

Likely building blocks:

```rust
pub trait DiagnosticSink: Send + Sync {
    fn emit(&self, diagnostic: &Diagnostic);
}

pub enum DiagnosticsMode {
    Disabled,
    Collect,
    Emit,
    CollectAndEmit,
}
```

Potential runtime container:

```rust
pub struct DiagnosticsContext {
    // sink, storage, configuration, counters
}
```

When multiple output renderers exist, the runtime should coordinate selection and fan-out rather than letting individual renderers race to claim the same output implicitly.

## Requirements

1. Low-level components must not print directly.
2. Emission must go through a configured runtime context or sink.
3. Collection must be supported without terminal output.
4. Emission must be order-preserving.
5. The runtime must be safe to use in tests and concurrent scenarios.
6. Diagnostics emitted during recovery must preserve the same ordering guarantees as diagnostics emitted before recovery.
7. If multiple output targets are configured, each target must have an explicit sink and renderer-selection path.
8. Common usage must be ergonomic with a minimal default runtime configuration.

## Standard Sink Modes

The runtime should support at least these behaviors:

- `NullSink`: ignore diagnostics
- `CollectingSink`: store diagnostics for later inspection
- `HookSink`: invoke user callback for each diagnostic
- `CompositeSink`: combine multiple sinks

For multi-renderer systems, `CompositeSink` is usually the right composition point. Each child sink can own one output target and select its renderer independently.

That composition should be easy to opt into, but not required for the simplest use cases.

## Tokenizer and Parser Sharing

The same context should be shareable across phases.

Example flow:

1. tokenizer emits warnings and recoverable errors
2. parser receives tokens and continues with the same diagnostics context
3. parser emits syntax diagnostics and recovery diagnostics into that same stream

This produces one coherent ordered diagnostic output for the full pipeline.

If the pipeline emits to multiple targets simultaneously, coherence should be preserved per target rather than by forcing one global renderer chain.

## Collection API

The runtime should support filtered access.

Examples:

- all diagnostics
- only errors
- only warnings
- by code
- by source subsystem

This matters because tokenizer today already exposes `last_errors`, and parser will likely need similar inspection helpers.

## Deduplication

Deduplication rules must be explicit.

Possible modes:

- no deduplication
- dedupe exact duplicates
- dedupe by code and span

Default behavior should probably avoid aggressive deduplication until real usage shows the need.

## Severity Policy

The runtime may later support policies such as:

- treat warnings as errors
- suppress selected codes
- downgrade selected severities

This is useful for CI, testing, and language-specific strictness modes.

## Runtime Configuration Guidance

Best practice is a layered API:

1. a simple default context for normal library or CLI use
2. a small set of high-value options for common overrides
3. advanced sink and renderer composition for integrators

This keeps the end-user experience minimal while still giving framework users real control.

## Open Questions

1. Should diagnostics context be mutable interior state or explicit return values?
2. Should sink hooks receive immutable diagnostics or a builder object before finalization?
3. Should ordering be strictly emission order or grouped by severity at render time only?
4. Should suppression and severity remapping belong to the runtime or a higher configuration layer?